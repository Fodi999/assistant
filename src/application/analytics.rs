use crate::shared::{AppError, AppResult};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;

const GOOGLE_AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const GOOGLE_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const GA4_SCOPE: &str = "https://www.googleapis.com/auth/analytics.readonly";

#[derive(Clone)]
pub struct AnalyticsService {
    client: Client,
    client_id: Option<String>,
    client_secret: Option<String>,
    redirect_uri: Option<String>,
    oauth_state: Option<String>,
    property_id: Option<String>,
    refresh_token: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AnalyticsOAuthUrl {
    pub url: String,
    pub redirect_uri: String,
    pub scope: String,
}

#[derive(Debug, Serialize)]
pub struct AnalyticsOAuthToken {
    pub refresh_token: Option<String>,
    pub access_token: Option<String>,
    pub expires_in: Option<i64>,
    pub scope: Option<String>,
    pub token_type: Option<String>,
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
        Self {
            client: Client::new(),
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
            refresh_token: non_empty_env("GA4_REFRESH_TOKEN"),
        }
    }

    pub fn oauth_url(&self) -> AppResult<AnalyticsOAuthUrl> {
        let client_id = self.required_client_id()?;
        let redirect_uri = self.required_redirect_uri()?;
        let state = self
            .oauth_state
            .clone()
            .unwrap_or_else(|| "ga4-admin-oauth".to_string());

        let url = format!(
            "{}?client_id={}&redirect_uri={}&response_type=code&scope={}&access_type=offline&prompt=consent&state={}",
            GOOGLE_AUTH_URL,
            percent_encode(&client_id),
            percent_encode(&redirect_uri),
            percent_encode(GA4_SCOPE),
            percent_encode(&state),
        );

        Ok(AnalyticsOAuthUrl {
            url,
            redirect_uri,
            scope: GA4_SCOPE.to_string(),
        })
    }

    pub async fn exchange_code(
        &self,
        code: &str,
        state: Option<&str>,
    ) -> AppResult<AnalyticsOAuthToken> {
        if let Some(expected) = self.oauth_state.as_deref() {
            if state != Some(expected) {
                return Err(AppError::authorization("Invalid Google OAuth state"));
            }
        }

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

        Ok(AnalyticsOAuthToken {
            refresh_token: token.refresh_token,
            access_token: token.access_token,
            expires_in: token.expires_in,
            scope: token.scope,
            token_type: token.token_type,
        })
    }

    pub async fn overview(&self, days: u16) -> AppResult<AnalyticsOverview> {
        let property_id = self.required_property_id()?;
        let access_token = self.access_token().await?;
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
        })
    }

    pub async fn realtime(&self) -> AppResult<AnalyticsRealtime> {
        let property_id = self.required_property_id()?;
        let access_token = self.access_token().await?;

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

    async fn access_token(&self) -> AppResult<String> {
        let client_id = self.required_client_id()?;
        let client_secret = self.required_client_secret()?;
        let refresh_token = self
            .refresh_token
            .as_deref()
            .ok_or_else(|| AppError::validation("GA4_REFRESH_TOKEN is not configured"))?;

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

    fn required_property_id(&self) -> AppResult<String> {
        self.property_id
            .clone()
            .ok_or_else(|| AppError::validation("GA4_PROPERTY_ID is not configured"))
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
