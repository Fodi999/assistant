use crate::shared::{AppError, AppResult};
use chrono::{Duration, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{PgPool, Row};
use time::OffsetDateTime;
use uuid::Uuid;

const GOOGLE_AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const GOOGLE_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const GOOGLE_SCOPES: &str = "https://www.googleapis.com/auth/analytics.readonly https://www.googleapis.com/auth/webmasters.readonly";

#[derive(Clone)]
pub struct AnalyticsService {
    client: Client,
    pool: Option<PgPool>,
    client_id: Option<String>,
    client_secret: Option<String>,
    redirect_uri: Option<String>,
    oauth_state: Option<String>,
    property_id: Option<String>,
    refresh_token: Option<String>,
    search_console_site_url: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AnalyticsOAuthUrl {
    pub url: String,
    pub redirect_uri: String,
    pub scope: String,
    pub site_id: String,
}

#[derive(Debug, Serialize)]
pub struct AnalyticsOAuthToken {
    pub site_id: String,
    pub refresh_token: Option<String>,
    pub access_token: Option<String>,
    pub expires_in: Option<i64>,
    pub scope: Option<String>,
    pub token_type: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AnalyticsConnectionStatus {
    pub site_id: String,
    pub status: String,
    pub google_property_id: Option<String>,
    pub connected_at: Option<OffsetDateTime>,
    pub has_refresh_token: bool,
    pub connection_id: Option<String>,
    pub source: String,
}

#[derive(Debug, Clone)]
struct SiteAnalyticsConfig {
    site_id: Uuid,
    google_property_id: Option<String>,
    refresh_token: Option<String>,
    connected_at: Option<OffsetDateTime>,
    status: String,
    connection_id: Option<String>,
    source: String,
}

#[derive(Debug, Serialize)]
pub struct AnalyticsOverview {
    pub configured: bool,
    pub property_id: Option<String>,
    pub date_range: String,
    pub active_users: f64,
    pub sessions: f64,
    pub page_views: f64,
    pub conversions: f64,
    pub total_revenue: f64,
    pub engagement_rate: f64,
    pub average_session_duration: f64,
    pub events: Vec<AnalyticsEventRow>,
    pub daily: Vec<AnalyticsDailyRow>,
    pub top_pages: Vec<AnalyticsPageRow>,
    pub countries: Vec<AnalyticsDimensionRow>,
    pub cities: Vec<AnalyticsDimensionRow>,
    pub regions: Vec<AnalyticsDimensionRow>,
    pub languages: Vec<AnalyticsDimensionRow>,
    pub devices: Vec<AnalyticsDimensionRow>,
    pub traffic_sources: Vec<AnalyticsDimensionRow>,
}

#[derive(Debug, Serialize)]
pub struct AnalyticsPageRow {
    pub path: String,
    pub title: String,
    pub views: f64,
    pub active_users: f64,
    pub engagement_rate: f64,
    pub conversions: f64,
    pub total_revenue: f64,
}

#[derive(Debug, Serialize)]
pub struct AnalyticsEventRow {
    pub event_name: String,
    pub count: f64,
    pub users: f64,
    pub total_revenue: f64,
}

#[derive(Debug, Serialize)]
pub struct AnalyticsDailyRow {
    pub date: String,
    pub active_users: f64,
    pub sessions: f64,
    pub page_views: f64,
    pub conversions: f64,
    pub total_revenue: f64,
}

#[derive(Debug, Serialize)]
pub struct AnalyticsDimensionRow {
    pub name: String,
    pub active_users: f64,
    pub sessions: f64,
    pub page_views: f64,
    pub conversions: f64,
}

#[derive(Debug, Serialize)]
pub struct AnalyticsRealtime {
    pub configured: bool,
    pub property_id: Option<String>,
    pub active_users: f64,
    pub pages: Vec<AnalyticsRealtimePage>,
    pub events: Vec<AnalyticsRealtimeEvent>,
}

#[derive(Debug, Serialize)]
pub struct AnalyticsRealtimePage {
    pub path: String,
    pub active_users: f64,
}

#[derive(Debug, Serialize)]
pub struct AnalyticsRealtimeEvent {
    pub event_name: String,
    pub active_users: f64,
    pub event_count: f64,
}

#[derive(Debug, Serialize)]
pub struct SearchConsoleSite {
    pub site_url: String,
    pub permission_level: String,
}

#[derive(Debug, Serialize)]
pub struct SearchConsoleOverview {
    pub configured: bool,
    pub site_url: String,
    pub date_range: String,
    pub clicks: f64,
    pub impressions: f64,
    pub ctr: f64,
    pub position: f64,
}

#[derive(Debug, Serialize)]
pub struct SearchConsoleRow {
    pub key: String,
    pub clicks: f64,
    pub impressions: f64,
    pub ctr: f64,
    pub position: f64,
}

#[derive(Debug, Serialize)]
pub struct SearchConsoleDailyRow {
    pub date: String,
    pub clicks: f64,
    pub impressions: f64,
    pub ctr: f64,
    pub position: f64,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: Option<String>,
    expires_in: Option<i64>,
    refresh_token: Option<String>,
    scope: Option<String>,
    token_type: Option<String>,
    error: Option<String>,
    error_description: Option<String>,
}

impl AnalyticsService {
    pub fn from_env() -> Self {
        Self::from_env_inner(None)
    }

    pub fn from_env_with_pool(pool: PgPool) -> Self {
        Self::from_env_inner(Some(pool))
    }

    fn from_env_inner(pool: Option<PgPool>) -> Self {
        Self {
            client: Client::new(),
            pool,
            client_id: first_non_empty_env(&["GOOGLE_OAUTH_CLIENT_ID", "GOOGLE_CLIENT_ID"]),
            client_secret: first_non_empty_env(&[
                "GOOGLE_OAUTH_CLIENT_SECRET",
                "GOOGLE_CLIENT_SECRET",
            ]),
            redirect_uri: first_non_empty_env(&[
                "GOOGLE_OAUTH_REDIRECT_URI",
                "GOOGLE_REDIRECT_URI",
            ]),
            oauth_state: non_empty_env("GA4_OAUTH_STATE")
                .or_else(|| Some("ga4-admin-oauth".to_string())),
            property_id: non_empty_env("GA4_PROPERTY_ID"),
            refresh_token: first_non_empty_env(&["GOOGLE_REFRESH_TOKEN", "GA4_REFRESH_TOKEN"]),
            search_console_site_url: first_non_empty_env(&[
                "SEARCH_CONSOLE_SITE_URL",
                "GSC_SITE_URL",
                "GOOGLE_SEARCH_CONSOLE_SITE_URL",
            ]),
        }
    }

    pub fn oauth_url(&self) -> AppResult<AnalyticsOAuthUrl> {
        self.oauth_url_for_site(default_analytics_site_id())
    }

    pub fn oauth_url_for_site(&self, site_id: Uuid) -> AppResult<AnalyticsOAuthUrl> {
        let client_id = self.required_client_id()?;
        let redirect_uri = self.required_redirect_uri()?;
        let state = self.oauth_state_for_site(site_id);

        let url = format!(
            "{}?client_id={}&redirect_uri={}&response_type=code&scope={}&access_type=offline&prompt=consent&state={}",
            GOOGLE_AUTH_URL,
            percent_encode(&client_id),
            percent_encode(&redirect_uri),
            percent_encode(GOOGLE_SCOPES),
            percent_encode(&state),
        );

        Ok(AnalyticsOAuthUrl {
            url,
            redirect_uri,
            scope: GOOGLE_SCOPES.to_string(),
            site_id: site_id.to_string(),
        })
    }

    pub async fn exchange_code(
        &self,
        code: &str,
        state: Option<&str>,
    ) -> AppResult<AnalyticsOAuthToken> {
        let site_id = self.site_id_from_oauth_state(state)?;

        let client_id = self.required_client_id()?;
        let client_secret = self.required_client_secret()?;
        let redirect_uri = self.required_redirect_uri()?;

        let response = self
            .client
            .post(GOOGLE_TOKEN_URL)
            .form(&[
                ("code", code),
                ("client_id", client_id.as_str()),
                ("client_secret", client_secret.as_str()),
                ("redirect_uri", redirect_uri.as_str()),
                ("grant_type", "authorization_code"),
            ])
            .send()
            .await
            .map_err(|e| AppError::internal(format!("Google OAuth request failed: {e}")))?;

        let status = response.status();
        let token: TokenResponse = response
            .json()
            .await
            .map_err(|e| AppError::internal(format!("Google OAuth response parse failed: {e}")))?;

        if !status.is_success() {
            return Err(AppError::validation(format!(
                "Google OAuth error: {}",
                token
                    .error_description
                    .or(token.error)
                    .unwrap_or_else(|| status.to_string())
            )));
        }

        let refresh_token = token.refresh_token.clone();
        if let Some(refresh_token) = refresh_token.as_deref() {
            self.save_site_refresh_token(site_id, refresh_token).await?;
        }

        Ok(AnalyticsOAuthToken {
            site_id: site_id.to_string(),
            refresh_token: token.refresh_token,
            access_token: token.access_token,
            expires_in: token.expires_in,
            scope: token.scope,
            token_type: token.token_type,
        })
    }

    pub async fn overview(&self, days: u16) -> AppResult<AnalyticsOverview> {
        self.overview_for_site(default_analytics_site_id(), days)
            .await
    }

    pub async fn overview_for_site(
        &self,
        site_id: Uuid,
        days: u16,
    ) -> AppResult<AnalyticsOverview> {
        let config = self.site_analytics_config(site_id).await?;
        let Some(property_id) = config.google_property_id.clone() else {
            return Ok(empty_analytics_overview(false, None, days));
        };
        let Some(refresh_token) = config.refresh_token.clone() else {
            return Ok(empty_analytics_overview(false, Some(property_id), days));
        };
        let access_token = match self.access_token_for_refresh_token(&refresh_token).await {
            Ok(access_token) => access_token,
            Err(error) => {
                self.mark_site_connection_status(site_id, analytics_error_status(&error))
                    .await?;
                return Err(error);
            }
        };
        let days = days.clamp(1, 365);
        let date_range = format!("{days}daysAgo");

        let summary = self
            .run_report(
                &property_id,
                &access_token,
                json!({
                    "dateRanges": [{ "startDate": date_range, "endDate": "today" }],
                    "metrics": [
                        { "name": "activeUsers" },
                        { "name": "sessions" },
                        { "name": "screenPageViews" },
                        { "name": "conversions" },
                        { "name": "totalRevenue" },
                        { "name": "engagementRate" },
                        { "name": "averageSessionDuration" }
                    ]
                }),
            )
            .await?;

        let pages = self
            .run_report(
                &property_id,
                &access_token,
                json!({
                    "dateRanges": [{ "startDate": format!("{days}daysAgo"), "endDate": "today" }],
                    "dimensions": [
                        { "name": "pagePath" },
                        { "name": "pageTitle" }
                    ],
                    "metrics": [
                        { "name": "screenPageViews" },
                        { "name": "activeUsers" },
                        { "name": "engagementRate" },
                        { "name": "conversions" },
                        { "name": "totalRevenue" }
                    ],
                    "orderBys": [{ "metric": { "metricName": "screenPageViews" }, "desc": true }],
                    "limit": 10
                }),
            )
            .await?;

        let events = self
            .run_report(
                &property_id,
                &access_token,
                json!({
                    "dateRanges": [{ "startDate": format!("{days}daysAgo"), "endDate": "today" }],
                    "dimensions": [{ "name": "eventName" }],
                    "metrics": [
                        { "name": "eventCount" },
                        { "name": "totalUsers" },
                        { "name": "totalRevenue" }
                    ],
                    "orderBys": [{ "metric": { "metricName": "eventCount" }, "desc": true }],
                    "limit": 25
                }),
            )
            .await?;

        let daily = self
            .run_report(
                &property_id,
                &access_token,
                json!({
                    "dateRanges": [{ "startDate": format!("{days}daysAgo"), "endDate": "today" }],
                    "dimensions": [{ "name": "date" }],
                    "metrics": [
                        { "name": "activeUsers" },
                        { "name": "sessions" },
                        { "name": "screenPageViews" },
                        { "name": "conversions" },
                        { "name": "totalRevenue" }
                    ],
                    "orderBys": [{ "dimension": { "dimensionName": "date" } }],
                    "limit": 400
                }),
            )
            .await?;

        let countries = self
            .run_dimension_report(&property_id, &access_token, days, "country", 10)
            .await?;
        let cities = self
            .run_dimension_report(&property_id, &access_token, days, "city", 10)
            .await?;
        let regions = self
            .run_dimension_report(&property_id, &access_token, days, "region", 10)
            .await?;
        let languages = self
            .run_dimension_report(&property_id, &access_token, days, "language", 10)
            .await?;
        let devices = self
            .run_dimension_report(&property_id, &access_token, days, "deviceCategory", 10)
            .await?;
        let traffic_sources = self
            .run_dimension_report(
                &property_id,
                &access_token,
                days,
                "sessionDefaultChannelGroup",
                10,
            )
            .await?;

        let metrics = summary
            .get("rows")
            .and_then(|v| v.as_array())
            .and_then(|rows| rows.first())
            .and_then(|row| row.get("metricValues"))
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        Ok(AnalyticsOverview {
            configured: true,
            property_id: Some(property_id),
            date_range: format!("last_{days}_days"),
            active_users: metric_value(&metrics, 0),
            sessions: metric_value(&metrics, 1),
            page_views: metric_value(&metrics, 2),
            conversions: metric_value(&metrics, 3),
            total_revenue: metric_value(&metrics, 4),
            engagement_rate: metric_value(&metrics, 5),
            average_session_duration: metric_value(&metrics, 6),
            events: parse_event_rows(&events),
            daily: parse_daily_rows(&daily),
            top_pages: parse_page_rows(&pages),
            countries: parse_dimension_rows(&countries),
            cities: parse_dimension_rows(&cities),
            regions: parse_dimension_rows(&regions),
            languages: parse_dimension_rows(&languages),
            devices: parse_dimension_rows(&devices),
            traffic_sources: parse_dimension_rows(&traffic_sources),
        })
    }

    pub async fn realtime(&self) -> AppResult<AnalyticsRealtime> {
        self.realtime_for_site(default_analytics_site_id()).await
    }

    pub async fn realtime_for_site(&self, site_id: Uuid) -> AppResult<AnalyticsRealtime> {
        let config = self.site_analytics_config(site_id).await?;
        let Some(property_id) = config.google_property_id.clone() else {
            return Ok(empty_analytics_realtime(false, None));
        };
        let Some(refresh_token) = config.refresh_token.clone() else {
            return Ok(empty_analytics_realtime(false, Some(property_id)));
        };
        let access_token = match self.access_token_for_refresh_token(&refresh_token).await {
            Ok(access_token) => access_token,
            Err(error) => {
                self.mark_site_connection_status(site_id, analytics_error_status(&error))
                    .await?;
                return Err(error);
            }
        };

        let active = self
            .run_realtime_report(
                &property_id,
                &access_token,
                json!({
                    "metrics": [{ "name": "activeUsers" }]
                }),
            )
            .await?;

        let pages = self
            .run_realtime_report(
                &property_id,
                &access_token,
                json!({
                    "dimensions": [{ "name": "unifiedScreenName" }],
                    "metrics": [{ "name": "screenPageViews" }],
                    "orderBys": [{ "metric": { "metricName": "screenPageViews" }, "desc": true }],
                    "limit": 10
                }),
            )
            .await
            .unwrap_or_else(|_| json!({ "rows": [] }));

        let events = self
            .run_realtime_report(
                &property_id,
                &access_token,
                json!({
                    "dimensions": [{ "name": "eventName" }],
                    "metrics": [{ "name": "eventCount" }],
                    "orderBys": [{ "metric": { "metricName": "eventCount" }, "desc": true }],
                    "limit": 20
                }),
            )
            .await
            .unwrap_or_else(|_| json!({ "rows": [] }));

        let metrics = active
            .get("rows")
            .and_then(|value| value.as_array())
            .and_then(|rows| rows.first())
            .and_then(|row| row.get("metricValues"))
            .and_then(|value| value.as_array())
            .cloned()
            .unwrap_or_default();

        Ok(AnalyticsRealtime {
            configured: true,
            property_id: Some(property_id),
            active_users: metric_value(&metrics, 0),
            pages: parse_realtime_pages(&pages),
            events: parse_realtime_events(&events),
        })
    }

    pub async fn search_console_sites(&self) -> AppResult<Vec<SearchConsoleSite>> {
        let access_token = self.access_token().await?;
        let response = self
            .client
            .get("https://www.googleapis.com/webmasters/v3/sites")
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(|e| AppError::internal(format!("Search Console sites request failed: {e}")))?;

        let status = response.status();
        let value: serde_json::Value = response.json().await.map_err(|e| {
            AppError::internal(format!("Search Console sites response parse failed: {e}"))
        })?;

        if !status.is_success() {
            return Err(AppError::validation(format!(
                "Search Console API error: {value}"
            )));
        }

        Ok(value
            .get("siteEntry")
            .and_then(|value| value.as_array())
            .map(|sites| {
                sites
                    .iter()
                    .map(|site| SearchConsoleSite {
                        site_url: site
                            .get("siteUrl")
                            .and_then(|value| value.as_str())
                            .unwrap_or_default()
                            .to_string(),
                        permission_level: site
                            .get("permissionLevel")
                            .and_then(|value| value.as_str())
                            .unwrap_or_default()
                            .to_string(),
                    })
                    .collect()
            })
            .unwrap_or_default())
    }

    pub async fn search_console_overview(
        &self,
        site_url: Option<String>,
        days: u16,
    ) -> AppResult<SearchConsoleOverview> {
        let site_url = self.required_search_console_site_url(site_url)?;
        let days = days.clamp(1, 365);
        let report = self
            .run_search_console_query(&site_url, days, json!({ "rowLimit": 1 }))
            .await?;
        let metrics = search_console_first_metrics(&report);

        Ok(SearchConsoleOverview {
            configured: true,
            site_url,
            date_range: format!("last_{days}_days"),
            clicks: metrics.0,
            impressions: metrics.1,
            ctr: metrics.2,
            position: metrics.3,
        })
    }

    pub async fn search_console_queries(
        &self,
        site_url: Option<String>,
        days: u16,
        limit: u16,
    ) -> AppResult<Vec<SearchConsoleRow>> {
        self.search_console_dimension(site_url, days, limit, "query")
            .await
    }

    pub async fn search_console_pages(
        &self,
        site_url: Option<String>,
        days: u16,
        limit: u16,
    ) -> AppResult<Vec<SearchConsoleRow>> {
        self.search_console_dimension(site_url, days, limit, "page")
            .await
    }

    pub async fn search_console_daily(
        &self,
        site_url: Option<String>,
        days: u16,
        limit: u16,
    ) -> AppResult<Vec<SearchConsoleDailyRow>> {
        let site_url = self.required_search_console_site_url(site_url)?;
        let report = self
            .run_search_console_query(
                &site_url,
                days.clamp(1, 365),
                json!({
                    "dimensions": ["date"],
                    "rowLimit": limit.clamp(1, 500)
                }),
            )
            .await?;

        Ok(parse_search_console_daily_rows(&report))
    }

    pub async fn connection_status(&self, site_id: Uuid) -> AppResult<AnalyticsConnectionStatus> {
        let config = self.site_analytics_config(site_id).await?;
        Ok(config.status_response())
    }

    pub async fn update_connection_property_id(
        &self,
        site_id: Uuid,
        google_property_id: Option<String>,
    ) -> AppResult<AnalyticsConnectionStatus> {
        let Some(pool) = &self.pool else {
            return Err(AppError::validation(
                "Analytics database config is not available",
            ));
        };

        let google_property_id = google_property_id
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());

        sqlx::query(
            r#"
            INSERT INTO site_analytics_connections
                (site_id, google_property_id, status, updated_at)
            VALUES ($1, $2, 'not_connected', NOW())
            ON CONFLICT (site_id) DO UPDATE
            SET google_property_id = EXCLUDED.google_property_id,
                status = CASE
                    WHEN site_analytics_connections.refresh_token IS NULL OR site_analytics_connections.refresh_token = '' THEN 'not_connected'
                    WHEN EXCLUDED.google_property_id IS NULL OR EXCLUDED.google_property_id = '' THEN 'error'
                    ELSE 'connected'
                END,
                updated_at = NOW()
            "#,
        )
        .bind(site_id)
        .bind(google_property_id)
        .execute(pool)
        .await?;

        self.connection_status(site_id).await
    }

    async fn site_analytics_config(&self, site_id: Uuid) -> AppResult<SiteAnalyticsConfig> {
        let row = if let Some(pool) = &self.pool {
            sqlx::query(
                r#"
                SELECT site_id, google_property_id, refresh_token, connection_id, connected_at, status
                FROM site_analytics_connections
                WHERE site_id = $1
                "#,
            )
            .bind(site_id)
            .fetch_optional(pool)
            .await?
        } else {
            None
        };

        let legacy_property_id = self.legacy_property_id_for_site(site_id);
        let legacy_refresh_token = self.legacy_refresh_token_for_site(site_id);

        let mut config = if let Some(row) = row {
            let google_property_id =
                non_empty(row.try_get::<Option<String>, _>("google_property_id")?)
                    .or(legacy_property_id);
            let refresh_token = non_empty(row.try_get::<Option<String>, _>("refresh_token")?)
                .or(legacy_refresh_token);
            let status = row
                .try_get::<String, _>("status")
                .unwrap_or_else(|_| "not_connected".to_string());

            SiteAnalyticsConfig {
                site_id,
                google_property_id,
                refresh_token,
                connected_at: row.try_get("connected_at").ok(),
                status,
                connection_id: non_empty(row.try_get::<Option<String>, _>("connection_id")?),
                source: "database".to_string(),
            }
        } else {
            SiteAnalyticsConfig {
                site_id,
                google_property_id: legacy_property_id,
                refresh_token: legacy_refresh_token,
                connected_at: None,
                status: "not_connected".to_string(),
                connection_id: None,
                source: "missing".to_string(),
            }
        };

        if config.source != "database"
            && config.google_property_id.is_some()
            && config.refresh_token.is_some()
        {
            config.status = "connected".to_string();
            config.source = "legacy_env".to_string();
        } else if config.status == "not_connected"
            && config.google_property_id.is_some()
            && config.refresh_token.is_some()
        {
            config.status = "connected".to_string();
        }

        Ok(config)
    }

    async fn save_site_refresh_token(&self, site_id: Uuid, refresh_token: &str) -> AppResult<()> {
        let Some(pool) = &self.pool else {
            return Ok(());
        };

        let property_id = self
            .site_analytics_config(site_id)
            .await
            .ok()
            .and_then(|config| config.google_property_id)
            .or_else(|| self.legacy_property_id_for_site(site_id));
        let connection_id = Uuid::new_v4().to_string();

        sqlx::query(
            r#"
            INSERT INTO site_analytics_connections
                (site_id, google_property_id, refresh_token, connection_id, connected_at, status, updated_at)
            VALUES (
                $1,
                $2,
                $3,
                $4,
                NOW(),
                CASE WHEN $2 IS NULL OR $2 = '' THEN 'error' ELSE 'connected' END,
                NOW()
            )
            ON CONFLICT (site_id) DO UPDATE
            SET google_property_id = COALESCE(EXCLUDED.google_property_id, site_analytics_connections.google_property_id),
                refresh_token = EXCLUDED.refresh_token,
                connection_id = EXCLUDED.connection_id,
                connected_at = EXCLUDED.connected_at,
                status = CASE
                    WHEN COALESCE(EXCLUDED.google_property_id, site_analytics_connections.google_property_id) IS NULL
                        OR COALESCE(EXCLUDED.google_property_id, site_analytics_connections.google_property_id) = '' THEN 'error'
                    ELSE 'connected'
                END,
                updated_at = NOW()
            "#,
        )
        .bind(site_id)
        .bind(property_id)
        .bind(refresh_token)
        .bind(connection_id)
        .execute(pool)
        .await?;

        Ok(())
    }

    async fn mark_site_connection_status(&self, site_id: Uuid, status: &str) -> AppResult<()> {
        let Some(pool) = &self.pool else {
            return Ok(());
        };

        sqlx::query(
            r#"
            UPDATE site_analytics_connections
            SET status = $2, updated_at = NOW()
            WHERE site_id = $1
            "#,
        )
        .bind(site_id)
        .bind(status)
        .execute(pool)
        .await?;

        Ok(())
    }

    async fn search_console_dimension(
        &self,
        site_url: Option<String>,
        days: u16,
        limit: u16,
        dimension: &str,
    ) -> AppResult<Vec<SearchConsoleRow>> {
        let site_url = self.required_search_console_site_url(site_url)?;
        let report = self
            .run_search_console_query(
                &site_url,
                days.clamp(1, 365),
                json!({
                    "dimensions": [dimension],
                    "rowLimit": limit.clamp(1, 250)
                }),
            )
            .await?;

        Ok(parse_search_console_rows(&report))
    }

    async fn access_token(&self) -> AppResult<String> {
        let refresh_token = self
            .refresh_token
            .as_deref()
            .ok_or_else(|| AppError::validation("GOOGLE_REFRESH_TOKEN is not configured"))?;

        self.access_token_for_refresh_token(refresh_token).await
    }

    async fn access_token_for_refresh_token(&self, refresh_token: &str) -> AppResult<String> {
        let client_id = self.required_client_id()?;
        let client_secret = self.required_client_secret()?;

        let response = self
            .client
            .post(GOOGLE_TOKEN_URL)
            .form(&[
                ("client_id", client_id.as_str()),
                ("client_secret", client_secret.as_str()),
                ("refresh_token", refresh_token),
                ("grant_type", "refresh_token"),
            ])
            .send()
            .await
            .map_err(|e| AppError::internal(format!("Google token refresh failed: {e}")))?;

        let status = response.status();
        let token: TokenResponse = response
            .json()
            .await
            .map_err(|e| AppError::internal(format!("Google token response parse failed: {e}")))?;

        if !status.is_success() {
            return Err(AppError::validation(format!(
                "Google token refresh error: {}",
                token
                    .error_description
                    .or(token.error)
                    .unwrap_or_else(|| status.to_string())
            )));
        }

        token
            .access_token
            .ok_or_else(|| AppError::internal("Google did not return access_token"))
    }

    async fn run_report(
        &self,
        property_id: &str,
        access_token: &str,
        body: serde_json::Value,
    ) -> AppResult<serde_json::Value> {
        let url = format!(
            "https://analyticsdata.googleapis.com/v1beta/properties/{}:runReport",
            property_id.trim_start_matches("properties/")
        );

        let response = self
            .client
            .post(url)
            .bearer_auth(access_token)
            .json(&body)
            .send()
            .await
            .map_err(|e| AppError::internal(format!("GA4 Data API request failed: {e}")))?;

        let status = response.status();
        let value: serde_json::Value = response
            .json()
            .await
            .map_err(|e| AppError::internal(format!("GA4 Data API response parse failed: {e}")))?;

        if !status.is_success() {
            return Err(AppError::validation(format!("GA4 Data API error: {value}")));
        }

        Ok(value)
    }

    async fn run_search_console_query(
        &self,
        site_url: &str,
        days: u16,
        body: serde_json::Value,
    ) -> AppResult<serde_json::Value> {
        let access_token = self.access_token().await?;
        let mut request_body = body;
        let start_date = days_ago_date(days);
        let end_date = today_date();

        if let Some(object) = request_body.as_object_mut() {
            object.insert("startDate".to_string(), json!(start_date));
            object.insert("endDate".to_string(), json!(end_date));
        }

        let url = format!(
            "https://www.googleapis.com/webmasters/v3/sites/{}/searchAnalytics/query",
            percent_encode(site_url)
        );

        let response = self
            .client
            .post(url)
            .bearer_auth(access_token)
            .json(&request_body)
            .send()
            .await
            .map_err(|e| AppError::internal(format!("Search Console query request failed: {e}")))?;

        let status = response.status();
        let value: serde_json::Value = response.json().await.map_err(|e| {
            AppError::internal(format!("Search Console query response parse failed: {e}"))
        })?;

        if !status.is_success() {
            return Err(AppError::validation(format!(
                "Search Console API error: {value}"
            )));
        }

        Ok(value)
    }

    async fn run_dimension_report(
        &self,
        property_id: &str,
        access_token: &str,
        days: u16,
        dimension: &str,
        limit: u16,
    ) -> AppResult<serde_json::Value> {
        self.run_report(
            property_id,
            access_token,
            json!({
                "dateRanges": [{ "startDate": format!("{days}daysAgo"), "endDate": "today" }],
                "dimensions": [{ "name": dimension }],
                "metrics": [
                    { "name": "activeUsers" },
                    { "name": "sessions" },
                    { "name": "screenPageViews" },
                    { "name": "conversions" }
                ],
                "orderBys": [{ "metric": { "metricName": "activeUsers" }, "desc": true }],
                "limit": limit
            }),
        )
        .await
    }

    async fn run_realtime_report(
        &self,
        property_id: &str,
        access_token: &str,
        body: serde_json::Value,
    ) -> AppResult<serde_json::Value> {
        let url = format!(
            "https://analyticsdata.googleapis.com/v1beta/properties/{}:runRealtimeReport",
            property_id.trim_start_matches("properties/")
        );

        let response = self
            .client
            .post(url)
            .bearer_auth(access_token)
            .json(&body)
            .send()
            .await
            .map_err(|e| AppError::internal(format!("GA4 Realtime API request failed: {e}")))?;

        let status = response.status();
        let value: serde_json::Value = response.json().await.map_err(|e| {
            AppError::internal(format!("GA4 Realtime API response parse failed: {e}"))
        })?;

        if !status.is_success() {
            return Err(AppError::validation(format!(
                "GA4 Realtime API error: {value}"
            )));
        }

        Ok(value)
    }

    fn required_client_id(&self) -> AppResult<String> {
        self.client_id
            .clone()
            .ok_or_else(|| AppError::validation("GOOGLE_OAUTH_CLIENT_ID is not configured"))
    }

    fn required_client_secret(&self) -> AppResult<String> {
        self.client_secret
            .clone()
            .ok_or_else(|| AppError::validation("GOOGLE_OAUTH_CLIENT_SECRET is not configured"))
    }

    fn required_redirect_uri(&self) -> AppResult<String> {
        self.redirect_uri
            .clone()
            .ok_or_else(|| AppError::validation("GOOGLE_OAUTH_REDIRECT_URI is not configured"))
    }

    fn legacy_property_id_for_site(&self, site_id: Uuid) -> Option<String> {
        if site_id == default_analytics_site_id() {
            self.property_id.clone()
        } else {
            None
        }
    }

    fn legacy_refresh_token_for_site(&self, site_id: Uuid) -> Option<String> {
        if site_id == default_analytics_site_id() {
            self.refresh_token.clone()
        } else {
            None
        }
    }

    fn oauth_state_base(&self) -> String {
        self.oauth_state
            .clone()
            .unwrap_or_else(|| "ga4-admin-oauth".to_string())
    }

    fn oauth_state_for_site(&self, site_id: Uuid) -> String {
        format!("{}:{site_id}", self.oauth_state_base())
    }

    fn site_id_from_oauth_state(&self, state: Option<&str>) -> AppResult<Uuid> {
        let Some(state) = state else {
            return Err(AppError::authorization("Invalid Google OAuth state"));
        };
        let base = self.oauth_state_base();

        if state == base {
            return Ok(default_analytics_site_id());
        }

        let Some((provided_base, site_id)) = state.rsplit_once(':') else {
            return Err(AppError::authorization("Invalid Google OAuth state"));
        };

        if provided_base != base {
            return Err(AppError::authorization("Invalid Google OAuth state"));
        }

        Uuid::parse_str(site_id).map_err(|_| AppError::authorization("Invalid Google OAuth state"))
    }

    fn required_search_console_site_url(&self, site_url: Option<String>) -> AppResult<String> {
        site_url
            .filter(|value| !value.trim().is_empty())
            .or_else(|| self.search_console_site_url.clone())
            .ok_or_else(|| {
                AppError::validation(
                    "SEARCH_CONSOLE_SITE_URL is not configured; call /api/admin/search-console/sites first",
                )
            })
    }
}

impl SiteAnalyticsConfig {
    fn status_response(&self) -> AnalyticsConnectionStatus {
        AnalyticsConnectionStatus {
            site_id: self.site_id.to_string(),
            status: self.status.clone(),
            google_property_id: self.google_property_id.clone(),
            connected_at: self.connected_at,
            has_refresh_token: self.refresh_token.is_some(),
            connection_id: self.connection_id.clone(),
            source: self.source.clone(),
        }
    }
}

fn default_analytics_site_id() -> Uuid {
    Uuid::from_u128(0x00000000000000000000000000000103)
}

fn empty_analytics_overview(
    configured: bool,
    property_id: Option<String>,
    days: u16,
) -> AnalyticsOverview {
    let days = days.clamp(1, 365);

    AnalyticsOverview {
        configured,
        property_id,
        date_range: format!("last_{days}_days"),
        active_users: 0.0,
        sessions: 0.0,
        page_views: 0.0,
        conversions: 0.0,
        total_revenue: 0.0,
        engagement_rate: 0.0,
        average_session_duration: 0.0,
        events: Vec::new(),
        daily: Vec::new(),
        top_pages: Vec::new(),
        countries: Vec::new(),
        cities: Vec::new(),
        regions: Vec::new(),
        languages: Vec::new(),
        devices: Vec::new(),
        traffic_sources: Vec::new(),
    }
}

fn empty_analytics_realtime(configured: bool, property_id: Option<String>) -> AnalyticsRealtime {
    AnalyticsRealtime {
        configured,
        property_id,
        active_users: 0.0,
        pages: Vec::new(),
        events: Vec::new(),
    }
}

fn non_empty(value: Option<String>) -> Option<String> {
    value.filter(|value| !value.trim().is_empty())
}

fn analytics_error_status(error: &AppError) -> &'static str {
    let message = error.to_string().to_ascii_lowercase();
    if message.contains("expired")
        || message.contains("revoked")
        || message.contains("invalid_grant")
    {
        "expired"
    } else {
        "error"
    }
}

fn non_empty_env(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .filter(|value| !value.trim().is_empty())
}

fn first_non_empty_env(keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| non_empty_env(key))
}

fn metric_value(values: &[serde_json::Value], index: usize) -> f64 {
    values
        .get(index)
        .and_then(|value| value.get("value"))
        .and_then(|value| value.as_str())
        .and_then(|value| value.parse::<f64>().ok())
        .unwrap_or(0.0)
}

fn json_number(value: &serde_json::Value, key: &str) -> f64 {
    value
        .get(key)
        .and_then(|value| value.as_f64())
        .unwrap_or(0.0)
}

fn search_console_first_metrics(report: &serde_json::Value) -> (f64, f64, f64, f64) {
    report
        .get("rows")
        .and_then(|value| value.as_array())
        .and_then(|rows| rows.first())
        .map(|row| {
            (
                json_number(row, "clicks"),
                json_number(row, "impressions"),
                json_number(row, "ctr"),
                json_number(row, "position"),
            )
        })
        .unwrap_or((0.0, 0.0, 0.0, 0.0))
}

fn parse_search_console_rows(report: &serde_json::Value) -> Vec<SearchConsoleRow> {
    report
        .get("rows")
        .and_then(|value| value.as_array())
        .map(|rows| {
            rows.iter()
                .map(|row| SearchConsoleRow {
                    key: search_console_key(row),
                    clicks: json_number(row, "clicks"),
                    impressions: json_number(row, "impressions"),
                    ctr: json_number(row, "ctr"),
                    position: json_number(row, "position"),
                })
                .collect()
        })
        .unwrap_or_default()
}

fn parse_search_console_daily_rows(report: &serde_json::Value) -> Vec<SearchConsoleDailyRow> {
    report
        .get("rows")
        .and_then(|value| value.as_array())
        .map(|rows| {
            rows.iter()
                .map(|row| SearchConsoleDailyRow {
                    date: search_console_key(row),
                    clicks: json_number(row, "clicks"),
                    impressions: json_number(row, "impressions"),
                    ctr: json_number(row, "ctr"),
                    position: json_number(row, "position"),
                })
                .collect()
        })
        .unwrap_or_default()
}

fn search_console_key(row: &serde_json::Value) -> String {
    row.get("keys")
        .and_then(|value| value.as_array())
        .and_then(|keys| keys.first())
        .and_then(|value| value.as_str())
        .unwrap_or_default()
        .to_string()
}

fn today_date() -> String {
    Utc::now().date_naive().format("%Y-%m-%d").to_string()
}

fn days_ago_date(days: u16) -> String {
    (Utc::now().date_naive() - Duration::days(days as i64))
        .format("%Y-%m-%d")
        .to_string()
}

fn parse_page_rows(report: &serde_json::Value) -> Vec<AnalyticsPageRow> {
    report
        .get("rows")
        .and_then(|value| value.as_array())
        .map(|rows| {
            rows.iter()
                .map(|row| {
                    let dimensions = row
                        .get("dimensionValues")
                        .and_then(|value| value.as_array())
                        .cloned()
                        .unwrap_or_default();
                    let metrics = row
                        .get("metricValues")
                        .and_then(|value| value.as_array())
                        .cloned()
                        .unwrap_or_default();

                    AnalyticsPageRow {
                        path: dimension_value(&dimensions, 0),
                        title: dimension_value(&dimensions, 1),
                        views: metric_value(&metrics, 0),
                        active_users: metric_value(&metrics, 1),
                        engagement_rate: metric_value(&metrics, 2),
                        conversions: metric_value(&metrics, 3),
                        total_revenue: metric_value(&metrics, 4),
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}

fn parse_event_rows(report: &serde_json::Value) -> Vec<AnalyticsEventRow> {
    report
        .get("rows")
        .and_then(|value| value.as_array())
        .map(|rows| {
            rows.iter()
                .map(|row| {
                    let dimensions = row
                        .get("dimensionValues")
                        .and_then(|value| value.as_array())
                        .cloned()
                        .unwrap_or_default();
                    let metrics = row
                        .get("metricValues")
                        .and_then(|value| value.as_array())
                        .cloned()
                        .unwrap_or_default();

                    AnalyticsEventRow {
                        event_name: dimension_value(&dimensions, 0),
                        count: metric_value(&metrics, 0),
                        users: metric_value(&metrics, 1),
                        total_revenue: metric_value(&metrics, 2),
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}

fn parse_daily_rows(report: &serde_json::Value) -> Vec<AnalyticsDailyRow> {
    report
        .get("rows")
        .and_then(|value| value.as_array())
        .map(|rows| {
            rows.iter()
                .map(|row| {
                    let dimensions = row
                        .get("dimensionValues")
                        .and_then(|value| value.as_array())
                        .cloned()
                        .unwrap_or_default();
                    let metrics = row
                        .get("metricValues")
                        .and_then(|value| value.as_array())
                        .cloned()
                        .unwrap_or_default();

                    AnalyticsDailyRow {
                        date: dimension_value(&dimensions, 0),
                        active_users: metric_value(&metrics, 0),
                        sessions: metric_value(&metrics, 1),
                        page_views: metric_value(&metrics, 2),
                        conversions: metric_value(&metrics, 3),
                        total_revenue: metric_value(&metrics, 4),
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}

fn parse_dimension_rows(report: &serde_json::Value) -> Vec<AnalyticsDimensionRow> {
    report
        .get("rows")
        .and_then(|value| value.as_array())
        .map(|rows| {
            rows.iter()
                .map(|row| {
                    let dimensions = row
                        .get("dimensionValues")
                        .and_then(|value| value.as_array())
                        .cloned()
                        .unwrap_or_default();
                    let metrics = row
                        .get("metricValues")
                        .and_then(|value| value.as_array())
                        .cloned()
                        .unwrap_or_default();

                    AnalyticsDimensionRow {
                        name: normalize_dimension_name(&dimension_value(&dimensions, 0)),
                        active_users: metric_value(&metrics, 0),
                        sessions: metric_value(&metrics, 1),
                        page_views: metric_value(&metrics, 2),
                        conversions: metric_value(&metrics, 3),
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}

fn parse_realtime_pages(report: &serde_json::Value) -> Vec<AnalyticsRealtimePage> {
    report
        .get("rows")
        .and_then(|value| value.as_array())
        .map(|rows| {
            rows.iter()
                .map(|row| {
                    let dimensions = row
                        .get("dimensionValues")
                        .and_then(|value| value.as_array())
                        .cloned()
                        .unwrap_or_default();
                    let metrics = row
                        .get("metricValues")
                        .and_then(|value| value.as_array())
                        .cloned()
                        .unwrap_or_default();

                    AnalyticsRealtimePage {
                        path: dimension_value(&dimensions, 0),
                        active_users: metric_value(&metrics, 0),
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}

fn parse_realtime_events(report: &serde_json::Value) -> Vec<AnalyticsRealtimeEvent> {
    report
        .get("rows")
        .and_then(|value| value.as_array())
        .map(|rows| {
            rows.iter()
                .map(|row| {
                    let dimensions = row
                        .get("dimensionValues")
                        .and_then(|value| value.as_array())
                        .cloned()
                        .unwrap_or_default();
                    let metrics = row
                        .get("metricValues")
                        .and_then(|value| value.as_array())
                        .cloned()
                        .unwrap_or_default();

                    AnalyticsRealtimeEvent {
                        event_name: dimension_value(&dimensions, 0),
                        active_users: 0.0,
                        event_count: metric_value(&metrics, 0),
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}

fn normalize_dimension_name(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed == "(not set)" {
        "Не определено".to_string()
    } else {
        trimmed.to_string()
    }
}

fn dimension_value(values: &[serde_json::Value], index: usize) -> String {
    values
        .get(index)
        .and_then(|value| value.get("value"))
        .and_then(|value| value.as_str())
        .unwrap_or_default()
        .to_string()
}

fn percent_encode(value: &str) -> String {
    value
        .bytes()
        .flat_map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                vec![byte as char]
            }
            _ => format!("%{byte:02X}").chars().collect(),
        })
        .collect()
}
