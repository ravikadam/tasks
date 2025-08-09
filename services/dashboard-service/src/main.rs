use axum::{
    extract::{Query, State},
    response::{Html, Json, Redirect},
    routing::{get, post},
    Router,
};
use axum_extra::extract::cookie::{Cookie, CookieJar};
use common::{config::ServiceConfig, http_client::HttpClient, HealthResponse, ServiceResult};
use models::{Task, RegisterRequest, LoginRequest, LoginResponse, UserProfile};
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tower_cookies::CookieManagerLayer;
use tracing::{info, instrument, error};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Mutex;

mod oauth;

#[derive(Clone)]
struct AppState {
    config: ServiceConfig,
    http_client: HttpClient,
    oauth_manager: Option<oauth::OAuthManager>,
    oauth_states: Arc<Mutex<HashMap<String, oauth::AuthState>>>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();

    let config = ServiceConfig::from_env("dashboard-service", 8006);

    tracing_subscriber::fmt()
        .with_env_filter(&config.log_level)
        .init();

    // Initialize OAuth manager if credentials are provided
    let oauth_manager = if let (Ok(client_id), Ok(client_secret), Ok(tenant_id)) = (
        std::env::var("AZURE_CLIENT_ID"),
        std::env::var("AZURE_CLIENT_SECRET"),
        std::env::var("AZURE_TENANT_ID"),
    ) {
        let oauth_config = oauth::OAuthConfig {
            client_id,
            client_secret,
            redirect_uri: format!("http://localhost:{}/oauth/callback", config.port),
            tenant_id,
        };
        match oauth::OAuthManager::new(oauth_config) {
            Ok(manager) => Some(manager),
            Err(e) => {
                error!("Failed to initialize OAuth manager: {}", e);
                None
            }
        }
    } else {
        info!("OAuth credentials not provided, email authentication disabled");
        None
    };

    let state = AppState {
        config: config.clone(),
        http_client: HttpClient::new(),
        oauth_manager,
        oauth_states: Arc::new(Mutex::new(HashMap::new())),
    };

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/", get(dashboard_home))
        .route("/login", get(show_login_page))
        .route("/register", get(show_register_page))
        .route("/api/auth/login", post(handle_login))
        .route("/api/auth/register", post(handle_register))
        .route("/api/auth/logout", post(handle_logout))
        .route("/dashboard", get(show_pending_tasks))
        .route("/config", get(show_config_page))
        .route("/ui/api/tasks", get(get_pending_tasks_api))
        .route("/oauth/login", get(oauth_login))
        .route("/oauth/callback", get(oauth_callback))
        .with_state(Arc::new(state))
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(CookieManagerLayer::new())
                .layer(CorsLayer::permissive()),
        );

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", config.port)).await?;
    info!("Dashboard Service listening on port {}", config.port);

    axum::serve(listener, app).await?;
    Ok(())
}

#[instrument]
async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse::new("dashboard-service"))
}

// Authentication helper functions
async fn get_current_user(state: &AppState, cookies: &CookieJar) -> Option<UserProfile> {
    let session_token = cookies.get("session_token")?.value();
    
    let url = format!("{}/api/v1/auth/validate", state.config.service_url("persistence"));
    let request = serde_json::json!({ "session_token": session_token });
    
    match state.http_client.post::<serde_json::Value, UserProfile>(&url, &request).await {
        Ok(user) => Some(user),
        Err(_) => None,
    }
}

// Route handlers
#[instrument(skip(state))]
async fn dashboard_home(
    State(state): State<Arc<AppState>>,
    cookies: CookieJar,
) -> ServiceResult<Html<String>> {
    if let Some(_user) = get_current_user(&state, &cookies).await {
        // User is logged in, redirect to dashboard
        Ok(Html(r#"<script>window.location.href = '/dashboard';</script>"#.to_string()))
    } else {
        // User not logged in, redirect to login
        Ok(Html(r#"<script>window.location.href = '/login';</script>"#.to_string()))
    }
}

#[instrument]
async fn show_login_page() -> Html<String> {
    let html = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="utf-8"/>
    <meta name="viewport" content="width=device-width, initial-scale=1"/>
    <title>Login • Task Manager</title>
    <script src="https://cdn.tailwindcss.com"></script>
</head>
<body class="min-h-screen bg-gradient-to-br from-blue-50 to-indigo-100">
    <div class="min-h-screen flex items-center justify-center py-12 px-4 sm:px-6 lg:px-8">
        <div class="max-w-md w-full space-y-8">
            <div class="text-center">
                <div class="mx-auto h-16 w-16 rounded-full bg-blue-600 text-white flex items-center justify-center text-2xl font-bold">TM</div>
                <h2 class="mt-6 text-3xl font-extrabold text-gray-900">Sign in to your account</h2>
                <p class="mt-2 text-sm text-gray-600">
                    Or <a href="/register" class="font-medium text-blue-600 hover:text-blue-500">create a new account</a>
                </p>
            </div>
            <form class="mt-8 space-y-6" id="loginForm">
                <div class="rounded-md shadow-sm -space-y-px">
                    <div>
                        <label for="email" class="sr-only">Email address</label>
                        <input id="email" name="email" type="email" required 
                               class="relative block w-full px-3 py-2 border border-gray-300 placeholder-gray-500 text-gray-900 rounded-t-md focus:outline-none focus:ring-blue-500 focus:border-blue-500 focus:z-10 sm:text-sm" 
                               placeholder="Email address">
                    </div>
                    <div>
                        <label for="password" class="sr-only">Password</label>
                        <input id="password" name="password" type="password" required 
                               class="relative block w-full px-3 py-2 border border-gray-300 placeholder-gray-500 text-gray-900 rounded-b-md focus:outline-none focus:ring-blue-500 focus:border-blue-500 focus:z-10 sm:text-sm" 
                               placeholder="Password">
                    </div>
                </div>
                <div>
                    <button type="submit" 
                            class="group relative w-full flex justify-center py-2 px-4 border border-transparent text-sm font-medium rounded-md text-white bg-blue-600 hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500">
                        Sign in
                    </button>
                </div>
                <div id="error-message" class="hidden text-red-600 text-sm text-center"></div>
            </form>
        </div>
    </div>
    
    <script>
        document.getElementById('loginForm').addEventListener('submit', async (e) => {
            e.preventDefault();
            const formData = new FormData(e.target);
            const data = {
                email: formData.get('email'),
                password: formData.get('password')
            };
            
            try {
                const response = await fetch('/api/auth/login', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify(data)
                });
                
                if (response.ok) {
                    window.location.href = '/dashboard';
                } else {
                    const error = await response.text();
                    document.getElementById('error-message').textContent = error || 'Login failed';
                    document.getElementById('error-message').classList.remove('hidden');
                }
            } catch (error) {
                document.getElementById('error-message').textContent = 'Network error';
                document.getElementById('error-message').classList.remove('hidden');
            }
        });
    </script>
</body>
</html>"#;
    Html(html.to_string())
}

#[instrument]
async fn show_register_page() -> Html<String> {
    let html = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="utf-8"/>
    <meta name="viewport" content="width=device-width, initial-scale=1"/>
    <title>Register • Task Manager</title>
    <script src="https://cdn.tailwindcss.com"></script>
</head>
<body class="min-h-screen bg-gradient-to-br from-blue-50 to-indigo-100">
    <div class="min-h-screen flex items-center justify-center py-12 px-4 sm:px-6 lg:px-8">
        <div class="max-w-md w-full space-y-8">
            <div class="text-center">
                <div class="mx-auto h-16 w-16 rounded-full bg-blue-600 text-white flex items-center justify-center text-2xl font-bold">TM</div>
                <h2 class="mt-6 text-3xl font-extrabold text-gray-900">Create your account</h2>
                <p class="mt-2 text-sm text-gray-600">
                    Or <a href="/login" class="font-medium text-blue-600 hover:text-blue-500">sign in to existing account</a>
                </p>
            </div>
            <form class="mt-8 space-y-6" id="registerForm">
                <div class="space-y-4">
                    <div>
                        <label for="full_name" class="block text-sm font-medium text-gray-700">Full Name</label>
                        <input id="full_name" name="full_name" type="text" required 
                               class="mt-1 block w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm placeholder-gray-400 focus:outline-none focus:ring-blue-500 focus:border-blue-500 sm:text-sm" 
                               placeholder="Your full name">
                    </div>
                    <div>
                        <label for="email" class="block text-sm font-medium text-gray-700">Email Address</label>
                        <input id="email" name="email" type="email" required 
                               class="mt-1 block w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm placeholder-gray-400 focus:outline-none focus:ring-blue-500 focus:border-blue-500 sm:text-sm" 
                               placeholder="your@email.com">
                    </div>
                    <div>
                        <label for="organization" class="block text-sm font-medium text-gray-700">Organization (Optional)</label>
                        <input id="organization" name="organization" type="text" 
                               class="mt-1 block w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm placeholder-gray-400 focus:outline-none focus:ring-blue-500 focus:border-blue-500 sm:text-sm" 
                               placeholder="Your company or organization">
                    </div>
                    <div>
                        <label for="password" class="block text-sm font-medium text-gray-700">Password</label>
                        <input id="password" name="password" type="password" required 
                               class="mt-1 block w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm placeholder-gray-400 focus:outline-none focus:ring-blue-500 focus:border-blue-500 sm:text-sm" 
                               placeholder="Choose a strong password">
                    </div>
                </div>
                <div>
                    <button type="submit" 
                            class="group relative w-full flex justify-center py-2 px-4 border border-transparent text-sm font-medium rounded-md text-white bg-blue-600 hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500">
                        Create Account
                    </button>
                </div>
                <div id="error-message" class="hidden text-red-600 text-sm text-center"></div>
            </form>
        </div>
    </div>
    
    <script>
        document.getElementById('registerForm').addEventListener('submit', async (e) => {
            e.preventDefault();
            const formData = new FormData(e.target);
            const data = {
                full_name: formData.get('full_name'),
                email: formData.get('email'),
                organization: formData.get('organization') || null,
                password: formData.get('password')
            };
            
            try {
                const response = await fetch('/api/auth/register', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify(data)
                });
                
                if (response.ok) {
                    window.location.href = '/login?registered=true';
                } else {
                    const error = await response.text();
                    document.getElementById('error-message').textContent = error || 'Registration failed';
                    document.getElementById('error-message').classList.remove('hidden');
                }
            } catch (error) {
                document.getElementById('error-message').textContent = 'Network error';
                document.getElementById('error-message').classList.remove('hidden');
            }
        });
    </script>
</body>
</html>"#;
    Html(html.to_string())
}

// Authentication API handlers
#[instrument(skip(state))]
async fn handle_register(
    State(state): State<Arc<AppState>>,
    Json(request): Json<RegisterRequest>,
) -> ServiceResult<Json<UserProfile>> {
    let url = format!("{}/api/v1/auth/register", state.config.service_url("persistence"));
    
    match state.http_client.post::<RegisterRequest, UserProfile>(&url, &request).await {
        Ok(user) => Ok(Json(user)),
        Err(e) => {
            error!("Registration failed: {}", e);
            Err(common::ServiceError::BadRequest("Registration failed".to_string()))
        }
    }
}

#[instrument(skip(state))]
async fn handle_login(
    State(state): State<Arc<AppState>>,
    cookies: CookieJar,
    Json(request): Json<LoginRequest>,
) -> ServiceResult<(CookieJar, Json<UserProfile>)> {
    let url = format!("{}/api/v1/auth/login", state.config.service_url("persistence"));
    
    match state.http_client.post::<LoginRequest, LoginResponse>(&url, &request).await {
        Ok(login_response) => {
            // Set session cookie
            let cookie = Cookie::build(("session_token", login_response.session_token))
                .path("/")
                .http_only(true)
                .max_age(tower_cookies::cookie::time::Duration::hours(24))
                .build();
            
            let updated_cookies = cookies.add(cookie);
            Ok((updated_cookies, Json(login_response.user)))
        }
        Err(e) => {
            error!("Login failed: {}", e);
            Err(common::ServiceError::Unauthorized("Invalid credentials".to_string()))
        }
    }
}

#[instrument]
async fn handle_logout(cookies: CookieJar) -> ServiceResult<(CookieJar, Json<serde_json::Value>)> {
    let cookie = Cookie::build(("session_token", ""))
        .path("/")
        .max_age(tower_cookies::cookie::time::Duration::seconds(0))
        .build();
    
    let updated_cookies = cookies.add(cookie);
    Ok((updated_cookies, Json(serde_json::json!({"message": "Logged out successfully"}))))
}

#[instrument(skip(state))]
async fn show_pending_tasks(
    State(state): State<Arc<AppState>>,
    cookies: CookieJar,
) -> ServiceResult<Html<String>> {
    // Check if user is authenticated
    let user = match get_current_user(&state, &cookies).await {
        Some(user) => user,
        None => {
            return Ok(Html(r#"<script>window.location.href = '/login';</script>"#.to_string()));
        }
    };
    let url = format!(
        "{}/api/v1/tasks?status=Pending",
        state.config.service_url("task-management")
    );
    let tasks = state
        .http_client
        .get::<Vec<Task>>(&url)
        .await
        .map_err(|e| common::ServiceError::HttpClient(e))?;

    let html = format!(r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="utf-8"/>
    <meta name="viewport" content="width=device-width, initial-scale=1"/>
    <title>Task Manager • Dashboard</title>
    <script src="https://cdn.tailwindcss.com"></script>
</head>
<body class="min-h-screen bg-gray-50">
    <div class="max-w-7xl mx-auto px-4 py-8">
        <header class="mb-8">
            <div class="flex items-center justify-between">
                <div class="flex items-center gap-3">
                    <div class="h-10 w-10 rounded-lg bg-blue-600 text-white flex items-center justify-center font-bold">TM</div>
                    <div>
                        <h1 class="text-2xl font-bold text-gray-900">Task Manager</h1>
                        <p class="text-gray-600">Welcome, {} • Dashboard</p>
                    </div>
                </div>
                <div class="flex items-center gap-3">
                    <div class="flex items-center gap-2 text-sm text-gray-600">
                        <div class="h-8 w-8 rounded-full bg-gray-300 flex items-center justify-center text-xs font-medium">
                            {}
                        </div>
                        <span>{}</span>
                    </div>
                    <a href="/config" class="bg-gray-600 hover:bg-gray-700 text-white rounded-md px-4 py-2 text-sm font-medium flex items-center gap-2">
                        ⚙️ Settings
                    </a>
                    <a href="/oauth/login" class="bg-green-600 hover:bg-green-700 text-white rounded-md px-4 py-2 text-sm font-medium flex items-center gap-2">
                        📧 Connect Email
                    </a>
                    <button onclick="location.reload()" class="bg-white border border-gray-300 rounded-md px-4 py-2 text-sm font-medium text-gray-700 hover:bg-gray-50">
                        Refresh
                    </button>
                    <button onclick="logout()" class="bg-red-600 hover:bg-red-700 text-white rounded-md px-4 py-2 text-sm font-medium">
                        Logout
                    </button>
                </div>
            </div>
        </header>
        
        <main>
            <div class="mb-6">
                <h2 class="text-3xl font-bold text-gray-900 mb-2">Pending Tasks</h2>
                <p class="text-gray-600 mb-4">You have {} pending tasks</p>
                <input type="search" placeholder="Search tasks..." class="w-full max-w-md border border-gray-300 rounded-lg px-4 py-2 text-sm focus:ring-2 focus:ring-blue-500 focus:border-transparent"/>
            </div>
            
            <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
                {}
            </div>
        </main>
    </div>
    
    <script>
        async function logout() {{
            try {{
                await fetch('/api/auth/logout', {{ method: 'POST' }});
                window.location.href = '/login';
            }} catch (error) {{
                console.error('Logout failed:', error);
                window.location.href = '/login';
            }}
        }}
    </script>
</body>
</html>"#, 
        user.full_name,
        user.full_name.chars().next().unwrap_or('U').to_uppercase().collect::<String>(),
        user.email,
        tasks.len(),
        if tasks.is_empty() {
            r#"<div class="col-span-full text-center py-12">
                <div class="text-gray-400 text-lg mb-2">🎉</div>
                <h3 class="text-lg font-medium text-gray-900 mb-1">All caught up!</h3>
                <p class="text-gray-500">No pending tasks right now.</p>
            </div>"#.to_string()
        } else {
            tasks.iter().map(|task| {
                format!(r#"<div class="bg-white rounded-xl border border-gray-200 p-6 shadow-sm hover:shadow-md transition-shadow">
                    <div class="flex items-start justify-between mb-4">
                        <h3 class="text-lg font-semibold text-gray-900">{}</h3>
                        <span class="bg-yellow-100 text-yellow-800 text-xs font-medium px-2.5 py-0.5 rounded-full">Pending</span>
                    </div>
                    <div class="flex items-center justify-between">
                        <div class="flex items-center text-sm text-gray-500">
                            <svg class="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z"/>
                            </svg>
                            Awaiting action
                        </div>
                        <button class="text-blue-600 hover:text-blue-700 text-sm font-medium">View</button>
                    </div>
                </div>"#, html_escape::encode_text(&task.title))
            }).collect::<Vec<_>>().join("")
        }
    );
    
    Ok(Html(html))
}

async fn show_config_page() -> Html<String> {
    let html = r#"<!DOCTYPE html>
    <html lang="en">
    <head>
        <meta charset="UTF-8">
        <meta name="viewport" content="width=device-width, initial-scale=1.0">
        <title>User Configuration</title>
        <script src="https://cdn.tailwindcss.com"></script>
    </head>
    <body class="bg-gray-50">
        <div class="min-h-screen flex items-center justify-center py-12 px-4 sm:px-6 lg:px-8">
            <div class="max-w-md w-full space-y-8">
                <div>
                    <h2 class="mt-6 text-center text-3xl font-extrabold text-gray-900">
                        User Configuration
                    </h2>
                    <p class="mt-2 text-center text-sm text-gray-600">
                        Manage your account settings and email connections
                    </p>
                </div>
                <div class="mt-8 space-y-6">
                    <div class="bg-white p-6 rounded-lg shadow">
                        <h3 class="text-lg font-medium text-gray-900 mb-4">Email Accounts</h3>
                        <button class="w-full flex justify-center py-2 px-4 border border-transparent rounded-md shadow-sm text-sm font-medium text-white bg-blue-600 hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500">
                            Add Email Account
                        </button>
                    </div>
                    <div class="bg-white p-6 rounded-lg shadow">
                        <h3 class="text-lg font-medium text-gray-900 mb-4">Database Settings</h3>
                        <p class="text-sm text-gray-600">Database configuration options will be available here.</p>
                    </div>
                    <div class="text-center">
                        <a href="/dashboard" class="text-blue-600 hover:text-blue-500">
                            Back to Dashboard
                        </a>
                    </div>
                </div>
            </div>
        </div>
    </body>
    </html>"#;
    Html(html.to_string())
}

#[instrument(skip(state))]
async fn get_pending_tasks_api(State(state): State<Arc<AppState>>) -> ServiceResult<Json<Vec<Task>>> {
    let url = format!(
        "{}/api/v1/tasks?status=Pending",
        state.config.service_url("task-management")
    );
    let tasks = state
        .http_client
        .get::<Vec<Task>>(&url)
        .await
        .map_err(|e| common::ServiceError::HttpClient(e))?;
    Ok(Json(tasks))
}

#[derive(Debug, Deserialize)]
struct CallbackQuery {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
}

#[instrument(skip(state))]
async fn oauth_login(State(state): State<Arc<AppState>>) -> ServiceResult<Redirect> {
    let oauth_manager = state.oauth_manager.as_ref()
        .ok_or_else(|| common::ServiceError::BadRequest("OAuth not configured".to_string()))?;

    let (auth_url, auth_state) = oauth_manager
        .get_authorization_url()
        .map_err(|e| common::ServiceError::Internal(e.into()))?;

    // Store the auth state for later retrieval in the callback
    if let Ok(mut states) = state.oauth_states.lock() {
        states.insert(auth_state.csrf_token.clone(), auth_state);
    }
    
    info!("OAuth login initiated, redirecting to: {}", auth_url);
    
    Ok(Redirect::to(auth_url.as_str()))
}

#[instrument(skip(state))]
async fn oauth_callback(
    State(state): State<Arc<AppState>>,
    Query(params): Query<CallbackQuery>,
) -> ServiceResult<Html<String>> {
    let oauth_manager = state.oauth_manager.as_ref()
        .ok_or_else(|| common::ServiceError::BadRequest("OAuth not configured".to_string()))?;

    if let Some(error) = params.error {
        return Ok(Html(format!(
            r#"
            <!DOCTYPE html>
            <html>
            <head>
                <title>OAuth Error</title>
                <script src="https://cdn.tailwindcss.com"></script>
            </head>
            <body class="bg-gray-50 flex items-center justify-center min-h-screen">
                <div class="bg-white p-8 rounded-lg shadow-md max-w-md w-full">
                    <h1 class="text-2xl font-bold text-red-600 mb-4">Authentication Error</h1>
                    <p class="text-gray-700 mb-4">OAuth authentication failed: {}</p>
                    <a href="/" class="bg-blue-500 hover:bg-blue-700 text-white font-bold py-2 px-4 rounded">
                        Back to Dashboard
                    </a>
                </div>
            </body>
            </html>
            "#,
            error
        )));
    }

    let code = params.code
        .ok_or_else(|| common::ServiceError::BadRequest("Missing authorization code".to_string()))?;

    // Retrieve the actual auth state using the state parameter
    let state_param = params.state
        .ok_or_else(|| common::ServiceError::BadRequest("Missing state parameter".to_string()))?;
    
    let auth_state = {
        let mut states = state.oauth_states.lock().map_err(|_| 
            common::ServiceError::Internal(anyhow::anyhow!("Failed to lock oauth states")))?;
        states.remove(&state_param)
            .ok_or_else(|| common::ServiceError::BadRequest("Invalid or expired state parameter".to_string()))?
    };

    match oauth_manager.exchange_code_for_token(code, auth_state).await {
        Ok(token_info) => {
            // Send token to email service
            if let Err(e) = oauth_manager.send_token_to_email_service(&token_info).await {
                error!("Failed to send token to email service: {}", e);
            }

            Ok(Html(format!(
                r#"
                <!DOCTYPE html>
                <html>
                <head>
                    <title>OAuth Success</title>
                    <script src="https://cdn.tailwindcss.com"></script>
                </head>
                <body class="bg-gray-50 flex items-center justify-center min-h-screen">
                    <div class="bg-white p-8 rounded-lg shadow-md max-w-md w-full">
                        <h1 class="text-2xl font-bold text-green-600 mb-4">✅ Authentication Successful!</h1>
                        <p class="text-gray-700 mb-4">Your Office 365 email account has been connected successfully. The email collector service will now be able to read your emails and create tasks automatically.</p>
                        <div class="space-y-2">
                            <a href="/" class="block w-full bg-blue-500 hover:bg-blue-700 text-white font-bold py-2 px-4 rounded text-center">
                                View Dashboard
                            </a>
                            <p class="text-sm text-gray-500 text-center">Email polling will start automatically</p>
                        </div>
                    </div>
                </body>
                </html>
                "#
            )))
        }
        Err(e) => {
            error!("Token exchange failed: {}", e);
            Ok(Html(format!(
                r#"
                <!DOCTYPE html>
                <html>
                <head>
                    <title>OAuth Error</title>
                    <script src="https://cdn.tailwindcss.com"></script>
                </head>
                <body class="bg-gray-50 flex items-center justify-center min-h-screen">
                    <div class="bg-white p-8 rounded-lg shadow-md max-w-md w-full">
                        <h1 class="text-2xl font-bold text-red-600 mb-4">Token Exchange Failed</h1>
                        <p class="text-gray-700 mb-4">Failed to exchange authorization code for token: {}</p>
                        <a href="/oauth/login" class="bg-blue-500 hover:bg-blue-700 text-white font-bold py-2 px-4 rounded">
                            Try Again
                        </a>
                    </div>
                </body>
                </html>
                "#,
                e
            )))
        }
    }
}
