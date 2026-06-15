use restaurant_backend::application::{
    ai_sous_chef::{
        use_cases::{
            create_product_draft::CreateDraftRequest, suggest_products::SuggestProductsRequest,
        },
        AiSousChefService,
    },
    intent_pages::IntentPagesService,
    public_seo_content::PublicSeoContentService,
    AdminCatalogService,
};
use restaurant_backend::infrastructure::{
    persistence::AiCacheRepository, GeminiService, LlmAdapter, R2Client, Repositories,
};
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgPoolOptions;
use std::{
    collections::BTreeMap,
    env, fs,
    path::{Component, Path, PathBuf},
    process,
    sync::Arc,
    time::Duration,
};
use uuid::Uuid;

type AnyResult<T> = Result<T, Box<dyn std::error::Error>>;

const USB_KEY_DIR_NAME: &str = "AssistantAdminKey";
const GEMINI_SETTINGS_PATH: &str = "settings/gemini.env.local";

#[tokio::main]
async fn main() -> AnyResult<()> {
    dotenvy::dotenv().ok();
    init_tracing();

    let mut args = env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty() || args[0] == "help" || args[0] == "--help" || args[0] == "-h" {
        print_help();
        return Ok(());
    }

    let command = args.remove(0);
    if is_usb_ai_command(&command) {
        let response = match handle_usb_ai_command(&command, args).await {
            Ok(value) => value,
            Err(error) => json_error(&command, error.to_string()),
        };
        print_json(&response)?;
        return Ok(());
    }

    let pool = connect_pool().await?;

    match command.as_str() {
        "state-audit" => {
            let service = AiSousChefService::new(pool);
            print_json(&service.state_audit().await?)?;
        }
        "data-quality" => {
            let service = AiSousChefService::new(pool);
            let rows = service.data_quality().await?;
            print_json(&rows)?;
        }
        "data-quality-one" => {
            let id = required_uuid(args.first(), "product_id")?;
            let service = AiSousChefService::new(pool);
            print_json(&service.data_quality_single(id).await?)?;
        }
        "generate-states" => {
            let id = required_uuid(args.first(), "product_id")?;
            let service = AiSousChefService::new(pool);
            print_json(&service.generate_states_for(id).await?)?;
        }
        "generate-states-all" => {
            let service = AiSousChefService::new(pool);
            print_json(&service.generate_all_states().await?)?;
        }
        "catalog-audit" => {
            let service = build_admin_catalog_service(pool).await;
            print_json(&service.ai_audit().await?)?;
        }
        "autofill-product" => {
            let id = required_uuid(args.first(), "product_id")?;
            let service = build_admin_catalog_service(pool).await;
            print_json(&service.ai_autofill(id).await?)?;
        }
        "generate-seo" => {
            let id = required_uuid(args.first(), "product_id")?;
            let service = build_admin_catalog_service(pool).await;
            print_json(&service.ai_generate_seo(id).await?)?;
        }
        "generate-pairings" => {
            let id = required_uuid(args.first(), "product_id")?;
            let service = build_admin_catalog_service(pool).await;
            print_json(&service.ai_generate_pairings(id).await?)?;
        }
        "suggest-products" => {
            let query = args
                .first()
                .map(String::as_str)
                .unwrap_or("Suggest useful missing products for a culinary catalog");
            let service = build_admin_catalog_service(pool).await;
            print_json(
                &service
                    .ai_suggest_products(SuggestProductsRequest {
                        query: query.to_string(),
                    })
                    .await?,
            )?;
        }
        "create-product-draft" => {
            let input = required_text(args.first(), "name_or_description")?;
            let service = build_admin_catalog_service(pool).await;
            print_json(
                &service
                    .ai_create_product_draft(CreateDraftRequest {
                        input: input.to_string(),
                    })
                    .await?,
            )?;
        }
        "generate-product-image" => {
            let name = required_text(args.first(), "name")?;
            let description = args.get(1).map(String::as_str);
            let service = build_admin_catalog_service(pool).await;
            let url = service
                .generate_product_draft_image(name, description, false)
                .await?;
            print_json(&serde_json::json!({ "image_url": url }))?;
        }
        "run-intent-scheduler" => {
            let service = build_intent_pages_service(pool).await;
            print_json(&service.run_scheduled_publish().await?)?;
        }
        other => {
            eprintln!("Unknown command: {other}\n");
            print_help();
            process::exit(2);
        }
    }

    Ok(())
}

async fn connect_pool() -> AnyResult<sqlx::PgPool> {
    let database_url =
        env::var("DATABASE_URL").map_err(|_| "DATABASE_URL is required for local admin tool")?;
    let max_connections = env::var("ADMIN_TOOL_MAX_DB_CONNECTIONS")
        .or_else(|_| env::var("MAX_DB_CONNECTIONS"))
        .ok()
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or(5);

    Ok(PgPoolOptions::new()
        .max_connections(max_connections)
        .acquire_timeout(Duration::from_secs(10))
        .connect(&database_url)
        .await?)
}

async fn build_admin_catalog_service(pool: sqlx::PgPool) -> AdminCatalogService {
    let repositories = Repositories::new(pool.clone());
    let r2_client = R2Client::new(
        env::var("CLOUDFLARE_ACCOUNT_ID").unwrap_or_default(),
        env::var("CLOUDFLARE_R2_ACCESS_KEY_ID").unwrap_or_default(),
        env::var("CLOUDFLARE_R2_SECRET_ACCESS_KEY").unwrap_or_default(),
        env::var("CLOUDFLARE_R2_BUCKET_NAME").unwrap_or_default(),
        env::var("CLOUDFLARE_R2_PUBLIC_URL").unwrap_or_default(),
    )
    .await;
    let gemini_service = Arc::new(GeminiService::new(
        env::var("GEMINI_API_KEY").unwrap_or_default(),
    ));
    let llm_adapter = Arc::new(LlmAdapter::new(
        gemini_service,
        Arc::new(repositories.ai_cache.clone()),
        Arc::new(repositories.ai_usage_stats.clone()),
    ));

    AdminCatalogService::new(
        pool,
        r2_client,
        repositories.dictionary.clone(),
        llm_adapter,
    )
}

async fn build_intent_pages_service(pool: sqlx::PgPool) -> Arc<IntentPagesService> {
    let repositories = Repositories::new(pool.clone());
    let llm_adapter = build_llm_adapter(&repositories);
    let seo_content = Arc::new(PublicSeoContentService::new(
        llm_adapter,
        AiCacheRepository::new(pool.clone()),
    ));
    Arc::new(IntentPagesService::new(
        pool,
        seo_content,
        build_r2_client().await,
    ))
}

fn build_llm_adapter(repositories: &Repositories) -> Arc<LlmAdapter> {
    let gemini_service = Arc::new(GeminiService::new(
        env::var("GEMINI_API_KEY").unwrap_or_default(),
    ));
    Arc::new(LlmAdapter::new(
        gemini_service,
        Arc::new(repositories.ai_cache.clone()),
        Arc::new(repositories.ai_usage_stats.clone()),
    ))
}

async fn build_r2_client() -> R2Client {
    R2Client::new(
        env::var("CLOUDFLARE_ACCOUNT_ID").unwrap_or_default(),
        env::var("CLOUDFLARE_R2_ACCESS_KEY_ID").unwrap_or_default(),
        env::var("CLOUDFLARE_R2_SECRET_ACCESS_KEY").unwrap_or_default(),
        env::var("CLOUDFLARE_R2_BUCKET_NAME").unwrap_or_default(),
        env::var("CLOUDFLARE_R2_PUBLIC_URL").unwrap_or_default(),
    )
    .await
}

fn is_usb_ai_command(command: &str) -> bool {
    matches!(
        command,
        "prompt-list"
            | "prompt-read"
            | "prompt-render"
            | "gemini-generate-text"
            | "gemini-generate-image-prompt"
            | "gemini-generate-product-card"
            | "gemini-generate-site-page"
            | "gemini-generate-blog-article"
            | "gemini-settings-status"
            | "ai-history-list"
            | "ai-history-read"
    )
}

async fn handle_usb_ai_command(command: &str, args: Vec<String>) -> AnyResult<serde_json::Value> {
    match command {
        "prompt-list" => prompt_list(command),
        "prompt-read" => {
            let path = flag_value(&args, "--path")?;
            prompt_read(command, path)
        }
        "prompt-render" => {
            let template = flag_value(&args, "--template")?;
            let vars = flag_value(&args, "--vars")?;
            prompt_render_command(command, template, vars)
        }
        "gemini-generate-text" => {
            let template = flag_value(&args, "--template")?;
            let vars = flag_value(&args, "--vars")?;
            gemini_generate_text(command, template, vars, "text_generation").await
        }
        "gemini-generate-image-prompt" => {
            let template = flag_value(&args, "--template")?;
            let vars = flag_value(&args, "--vars")?;
            gemini_generate_image_prompt(command, template, vars).await
        }
        "gemini-generate-product-card" => {
            let vars = flag_value(&args, "--vars")?;
            gemini_generate_text(
                command,
                "templates/prompts/sites/marketplace-product.ru.txt",
                vars,
                "product_card",
            )
            .await
        }
        "gemini-generate-site-page" => {
            let vars = flag_value(&args, "--vars")?;
            gemini_generate_text(
                command,
                "templates/prompts/sites/construction-site.ru.txt",
                vars,
                "site_page",
            )
            .await
        }
        "gemini-generate-blog-article" => {
            let vars = flag_value(&args, "--vars")?;
            gemini_generate_text(
                command,
                "templates/prompts/sites/blog-article.ru.txt",
                vars,
                "blog_article",
            )
            .await
        }
        "gemini-settings-status" => gemini_settings_status(command),
        "ai-history-list" => ai_history_list(command),
        "ai-history-read" => {
            let id = flag_value(&args, "--id")?;
            ai_history_read(command, id)
        }
        _ => Ok(json_error(command, "Unknown USB AI command")),
    }
}

fn flag_value<'a>(args: &'a [String], flag: &str) -> AnyResult<&'a str> {
    args.windows(2)
        .find(|pair| pair[0] == flag)
        .map(|pair| pair[1].as_str())
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("{flag} is required").into())
}

fn json_error(command: &str, error: impl Into<String>) -> serde_json::Value {
    serde_json::json!({
        "ok": false,
        "command": command,
        "error": error.into()
    })
}

fn find_usb_root() -> AnyResult<PathBuf> {
    for key in ["USB_ROOT", "ASSISTANT_ADMIN_KEY_ROOT"] {
        if let Ok(value) = env::var(key) {
            let path = PathBuf::from(value);
            if path.is_dir()
                && path.file_name().and_then(|name| name.to_str()) == Some(USB_KEY_DIR_NAME)
            {
                return Ok(path);
            }
        }
    }

    let volumes = PathBuf::from("/Volumes");
    let entries = fs::read_dir(&volumes).map_err(|_| "AssistantAdminKey not found in /Volumes")?;
    for entry in entries.flatten() {
        let candidate = entry.path().join(USB_KEY_DIR_NAME);
        if candidate.is_dir() {
            return Ok(candidate);
        }
    }

    Err("AssistantAdminKey not found in /Volumes".into())
}

fn safe_usb_path(root: &Path, relative: &str) -> AnyResult<PathBuf> {
    let path = Path::new(relative);
    if path.is_absolute() {
        return Err("Path must be relative to USB root".into());
    }
    if path.components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    }) {
        return Err("Path cannot contain parent directory components".into());
    }
    Ok(root.join(path))
}

fn prompt_type(path: &Path) -> &'static str {
    let text = path.to_string_lossy();
    if text.contains("/sites/") {
        "site"
    } else if text.contains("/images/") {
        "image"
    } else if text.contains("/system/") {
        "system"
    } else {
        "unknown"
    }
}

fn prompt_list(command: &str) -> AnyResult<serde_json::Value> {
    let root = find_usb_root()?;
    let base = root.join("templates/prompts");
    let mut prompts = Vec::new();
    collect_prompt_files(&root, &base, &mut prompts)?;
    prompts.sort_by(|left, right| {
        left["path"]
            .as_str()
            .unwrap_or_default()
            .cmp(right["path"].as_str().unwrap_or_default())
    });
    Ok(serde_json::json!({
        "ok": true,
        "command": command,
        "usb_root": root,
        "prompts": prompts
    }))
}

fn collect_prompt_files(
    root: &Path,
    dir: &Path,
    prompts: &mut Vec<serde_json::Value>,
) -> AnyResult<()> {
    if !dir.is_dir() {
        return Ok(());
    }
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path
            .file_name()
            .and_then(|name| name.to_str())
            .map(|name| name.starts_with('.'))
            .unwrap_or(false)
        {
            continue;
        }
        if path.is_dir() {
            collect_prompt_files(root, &path, prompts)?;
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("txt") {
            let relative = path.strip_prefix(root)?.to_string_lossy().to_string();
            prompts.push(serde_json::json!({
                "name": path.file_name().and_then(|name| name.to_str()).unwrap_or_default(),
                "path": relative,
                "type": prompt_type(&path)
            }));
        }
    }
    Ok(())
}

fn prompt_read(command: &str, relative_path: &str) -> AnyResult<serde_json::Value> {
    let root = find_usb_root()?;
    let path = safe_usb_path(&root, relative_path)?;
    let content = fs::read_to_string(&path)?;
    Ok(serde_json::json!({
        "ok": true,
        "command": command,
        "path": relative_path,
        "content": content
    }))
}

fn prompt_render_command(
    command: &str,
    template: &str,
    vars_path: &str,
) -> AnyResult<serde_json::Value> {
    let root = find_usb_root()?;
    let vars = read_vars_file(vars_path)?;
    let rendered = render_prompt_from_usb(&root, template, &vars)?;
    Ok(serde_json::json!({
        "ok": true,
        "command": command,
        "template": template,
        "prompt": rendered
    }))
}

fn read_vars_file(vars_path: &str) -> AnyResult<serde_json::Map<String, serde_json::Value>> {
    let content = fs::read_to_string(vars_path)?;
    let value: serde_json::Value = serde_json::from_str(&content)?;
    value
        .as_object()
        .cloned()
        .ok_or_else(|| "vars JSON must be an object".into())
}

fn render_prompt_from_usb(
    root: &Path,
    template: &str,
    vars: &serde_json::Map<String, serde_json::Value>,
) -> AnyResult<String> {
    let template_path = safe_usb_path(root, template)?;
    let content = fs::read_to_string(template_path)?;
    render_prompt(&content, vars)
}

fn render_prompt(
    content: &str,
    vars: &serde_json::Map<String, serde_json::Value>,
) -> AnyResult<String> {
    let variables = extract_template_variables(content);
    let mut missing = Vec::new();
    let mut rendered = content.to_string();
    for variable in variables {
        let Some(value) = vars.get(&variable) else {
            missing.push(variable);
            continue;
        };
        let replacement = match value {
            serde_json::Value::String(text) => text.clone(),
            other => other.to_string(),
        };
        rendered = rendered.replace(&format!("{{{{{variable}}}}}"), &replacement);
    }

    if !missing.is_empty() {
        return Err(serde_json::json!({ "missing_variables": missing })
            .to_string()
            .into());
    }

    Ok(rendered)
}

fn extract_template_variables(content: &str) -> Vec<String> {
    let mut variables = Vec::new();
    let mut rest = content;
    while let Some(start) = rest.find("{{") {
        let after_start = &rest[start + 2..];
        let Some(end) = after_start.find("}}") else {
            break;
        };
        let name = after_start[..end].trim();
        if !name.is_empty()
            && name
                .chars()
                .all(|char| char.is_ascii_uppercase() || char.is_ascii_digit() || char == '_')
            && !variables.iter().any(|existing| existing == name)
        {
            variables.push(name.to_string());
        }
        rest = &after_start[end + 2..];
    }
    variables
}

#[derive(Debug, Deserialize)]
struct GeminiResponse {
    candidates: Option<Vec<GeminiCandidate>>,
}

#[derive(Debug, Deserialize)]
struct GeminiCandidate {
    content: Option<GeminiContent>,
}

#[derive(Debug, Deserialize, Serialize)]
struct GeminiContent {
    parts: Option<Vec<GeminiPart>>,
}

#[derive(Debug, Deserialize, Serialize)]
struct GeminiPart {
    text: Option<String>,
}

fn read_gemini_settings(root: &Path) -> AnyResult<BTreeMap<String, String>> {
    let path = root.join(GEMINI_SETTINGS_PATH);
    if !path.is_file() {
        return Err("GEMINI_API_KEY is missing in settings/gemini.env.local".into());
    }

    let mut values = BTreeMap::new();
    for raw_line in fs::read_to_string(path)?.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            values.insert(key.trim().to_string(), value.trim().to_string());
        }
    }
    Ok(values)
}

async fn call_gemini_text(api_key: &str, model: &str, prompt: &str) -> AnyResult<String> {
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent?key={api_key}"
    );
    let client = reqwest::Client::new();
    let response = client
        .post(url)
        .json(&serde_json::json!({
            "contents": [{
                "parts": [{ "text": prompt }]
            }]
        }))
        .send()
        .await?;
    let status = response.status();
    let body = response.text().await?;
    if !status.is_success() {
        return Err(format!("Gemini request failed with status {status}: {body}").into());
    }
    let parsed: GeminiResponse = serde_json::from_str(&body)?;
    let text = parsed
        .candidates
        .and_then(|items| items.into_iter().next())
        .and_then(|candidate| candidate.content)
        .and_then(|content| content.parts)
        .and_then(|parts| parts.into_iter().find_map(|part| part.text))
        .ok_or("Gemini returned empty text")?;
    Ok(text)
}

async fn gemini_generate_text(
    command: &str,
    template: &str,
    vars_path: &str,
    generation_type: &str,
) -> AnyResult<serde_json::Value> {
    let root = find_usb_root()?;
    let settings = read_gemini_settings(&root)?;
    let api_key = settings
        .get("GEMINI_API_KEY")
        .filter(|value| !value.trim().is_empty())
        .ok_or("GEMINI_API_KEY is missing in settings/gemini.env.local")?;
    let model = settings
        .get("GEMINI_TEXT_MODEL")
        .filter(|value| !value.trim().is_empty())
        .map(String::as_str)
        .unwrap_or("gemini-3-flash-preview");
    let vars = read_vars_file(vars_path)?;
    let rendered_prompt = render_prompt_from_usb(&root, template, &vars)?;
    let text = call_gemini_text(api_key, model, &rendered_prompt).await?;
    let result = serde_json::json!({ "text": text });
    let (id, history_path, export_path) = save_generation(
        &root,
        generation_type,
        template,
        &vars,
        &rendered_prompt,
        model,
        &result,
    )?;

    Ok(serde_json::json!({
        "ok": true,
        "command": command,
        "generation_id": id,
        "template": template,
        "model": model,
        "output_path": history_path,
        "export_path": export_path,
        "preview": text.chars().take(700).collect::<String>()
    }))
}

async fn gemini_generate_image_prompt(
    command: &str,
    template: &str,
    vars_path: &str,
) -> AnyResult<serde_json::Value> {
    let root = find_usb_root()?;
    let settings = read_gemini_settings(&root).unwrap_or_default();
    let model = settings
        .get("GEMINI_TEXT_MODEL")
        .filter(|value| !value.trim().is_empty())
        .map(String::as_str)
        .unwrap_or("local-template-render");
    let vars = read_vars_file(vars_path)?;
    let rendered_prompt = render_prompt_from_usb(&root, template, &vars)?;
    let result = serde_json::json!({
        "type": "image_prompt",
        "prompt": rendered_prompt,
        "negative_prompt": "blurry, low quality, distorted geometry, extra text, watermark, logo artifacts, unrealistic proportions",
        "style": vars.get("STYLE").and_then(|value| value.as_str()).unwrap_or("commercial product photography"),
        "recommended_size": "1024x1024"
    });
    let (id, history_path, _export_path) = save_generation(
        &root,
        "image_prompt",
        template,
        &vars,
        &rendered_prompt,
        model,
        &result,
    )?;

    Ok(serde_json::json!({
        "ok": true,
        "command": command,
        "generation_id": id,
        "type": "image_prompt",
        "prompt": result["prompt"],
        "negative_prompt": result["negative_prompt"],
        "style": result["style"],
        "recommended_size": result["recommended_size"],
        "history_path": history_path
    }))
}

fn save_generation(
    root: &Path,
    generation_type: &str,
    template: &str,
    vars: &serde_json::Map<String, serde_json::Value>,
    rendered_prompt: &str,
    model: &str,
    result: &serde_json::Value,
) -> AnyResult<(String, String, String)> {
    let now = chrono::Utc::now();
    let timestamp = now.format("%Y%m%d%H%M%S").to_string();
    let id = format!("{}-{}", timestamp, Uuid::new_v4());
    let history_dir = if generation_type == "image_prompt" {
        root.join("data/ai-history/image-generations")
    } else {
        root.join("data/ai-history/text-generations")
    };
    let export_dir = root.join("exports/generated/json");
    fs::create_dir_all(&history_dir)?;
    fs::create_dir_all(&export_dir)?;

    let history_path = history_dir.join(format!("{timestamp}-{generation_type}.json"));
    let export_path = export_dir.join(format!("{timestamp}-result.json"));
    let history = serde_json::json!({
        "id": id,
        "created_at": now.to_rfc3339(),
        "type": generation_type,
        "template": template,
        "vars": vars,
        "rendered_prompt": rendered_prompt,
        "model": model,
        "result": result,
        "export_path": export_path
    });

    fs::write(&history_path, serde_json::to_vec_pretty(&history)?)?;
    fs::write(&export_path, serde_json::to_vec_pretty(result)?)?;
    append_usb_log(
        root,
        &format!("saved generation {id} type={generation_type}"),
    )?;

    Ok((
        id,
        history_path.display().to_string(),
        export_path.display().to_string(),
    ))
}

fn append_usb_log(root: &Path, message: &str) -> AnyResult<()> {
    let logs_dir = root.join("logs");
    fs::create_dir_all(&logs_dir)?;
    let line = format!("{} {}\n", chrono::Utc::now().to_rfc3339(), message);
    let log_path = logs_dir.join("gemini-generator.log");
    let mut previous = fs::read_to_string(&log_path).unwrap_or_default();
    previous.push_str(&line);
    fs::write(log_path, previous)?;
    Ok(())
}

fn gemini_settings_status(command: &str) -> AnyResult<serde_json::Value> {
    let root = find_usb_root()?;
    let path = root.join(GEMINI_SETTINGS_PATH);
    let settings = read_gemini_settings(&root).unwrap_or_default();
    Ok(serde_json::json!({
        "ok": true,
        "command": command,
        "usb_root": root,
        "settings_path": path,
        "gemini_api_key": if settings.get("GEMINI_API_KEY").map(|value| !value.trim().is_empty()).unwrap_or(false) { "configured" } else { "missing" },
        "text_model": settings.get("GEMINI_TEXT_MODEL").cloned().unwrap_or_else(|| "gemini-3-flash-preview".to_string()),
        "image_model": settings.get("GEMINI_IMAGE_MODEL").cloned().unwrap_or_default()
    }))
}

fn ai_history_list(command: &str) -> AnyResult<serde_json::Value> {
    let root = find_usb_root()?;
    let mut items = Vec::new();
    for dir in [
        root.join("data/ai-history/text-generations"),
        root.join("data/ai-history/image-generations"),
    ] {
        if !dir.is_dir() {
            continue;
        }
        for entry in fs::read_dir(dir)? {
            let path = entry?.path();
            if is_hidden_file(&path) {
                continue;
            }
            if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
                continue;
            }
            let value: serde_json::Value = serde_json::from_str(&fs::read_to_string(&path)?)?;
            items.push(serde_json::json!({
                "id": value["id"],
                "created_at": value["created_at"],
                "type": value["type"],
                "template": value["template"],
                "path": path
            }));
        }
    }
    items.sort_by(|left, right| {
        right["created_at"]
            .as_str()
            .unwrap_or_default()
            .cmp(left["created_at"].as_str().unwrap_or_default())
    });
    Ok(serde_json::json!({
        "ok": true,
        "command": command,
        "history": items
    }))
}

fn ai_history_read(command: &str, id: &str) -> AnyResult<serde_json::Value> {
    let root = find_usb_root()?;
    for dir in [
        root.join("data/ai-history/text-generations"),
        root.join("data/ai-history/image-generations"),
    ] {
        if !dir.is_dir() {
            continue;
        }
        for entry in fs::read_dir(dir)? {
            let path = entry?.path();
            if is_hidden_file(&path) {
                continue;
            }
            if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
                continue;
            }
            let value: serde_json::Value = serde_json::from_str(&fs::read_to_string(&path)?)?;
            if value["id"].as_str() == Some(id) {
                return Ok(serde_json::json!({
                    "ok": true,
                    "command": command,
                    "generation": value
                }));
            }
        }
    }
    Ok(json_error(command, format!("History item not found: {id}")))
}

fn is_hidden_file(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.starts_with('.'))
        .unwrap_or(false)
}

fn required_uuid(value: Option<&String>, name: &str) -> AnyResult<Uuid> {
    let raw = required_text(value, name)?;
    Ok(Uuid::parse_str(raw).map_err(|e| format!("Invalid {name}: {e}"))?)
}

fn required_text<'a>(value: Option<&'a String>, name: &str) -> AnyResult<&'a str> {
    value
        .map(String::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("{name} is required").into())
}

fn print_json<T: serde::Serialize>(value: &T) -> AnyResult<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

fn init_tracing() {
    let filter = env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .compact()
        .init();
}

fn print_help() {
    println!(
        r#"Local admin tool

Usage:
  cargo run --bin admin_tool -- <command> [args]

Commands:
  state-audit
  data-quality
  data-quality-one <product_id>
  generate-states <product_id>
  generate-states-all
  catalog-audit
  autofill-product <product_id>
  generate-seo <product_id>
  generate-pairings <product_id>
  suggest-products [category_or_context]
  create-product-draft <name_or_description>
  generate-product-image <name> [description]
  run-intent-scheduler
  prompt-list
  prompt-read --path <relative_prompt_path>
  prompt-render --template <relative_prompt_path> --vars <json_file>
  gemini-generate-text --template <relative_prompt_path> --vars <json_file>
  gemini-generate-image-prompt --template <relative_prompt_path> --vars <json_file>
  gemini-generate-product-card --vars <json_file>
  gemini-generate-site-page --vars <json_file>
  gemini-generate-blog-article --vars <json_file>
  gemini-settings-status
  ai-history-list
  ai-history-read --id <generation_id>

Required env:
  DATABASE_URL

AI commands also need:
  GEMINI_API_KEY

USB Gemini commands read:
  /Volumes/*/AssistantAdminKey/settings/gemini.env.local

Image upload commands also need:
  CLOUDFLARE_ACCOUNT_ID
  CLOUDFLARE_R2_ACCESS_KEY_ID
  CLOUDFLARE_R2_SECRET_ACCESS_KEY
  CLOUDFLARE_R2_BUCKET_NAME
  CLOUDFLARE_R2_PUBLIC_URL
"#
    );
}
