use crate::domain::{DisplayName, Email, Password, RefreshToken, Tenant, TenantName, User, UserRole};
use crate::infrastructure::{
    JwtService, PasswordHasher, RefreshTokenRepository, RefreshTokenRepositoryTrait,
    TenantRepository, TenantRepositoryTrait, UserRepository, UserRepositoryTrait,
};
use crate::shared::{AppError, AppResult, Language, TenantId, UserId};
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use time::OffsetDateTime;

#[derive(Clone)]
pub struct AuthService {
    user_repo: UserRepository,
    tenant_repo: TenantRepository,
    refresh_token_repo: RefreshTokenRepository,
    password_hasher: PasswordHasher,
    jwt_service: JwtService,
}

impl AuthService {
    pub fn new(
        user_repo: UserRepository,
        tenant_repo: TenantRepository,
        refresh_token_repo: RefreshTokenRepository,
        password_hasher: PasswordHasher,
        jwt_service: JwtService,
    ) -> Self {
        Self {
            user_repo,
            tenant_repo,
            refresh_token_repo,
            password_hasher,
            jwt_service,
        }
    }

    pub async fn register(&self, command: RegisterCommand) -> AppResult<AuthResponse> {
        // Validate input
        let email = Email::new(command.email)?;
        let password = Password::new(command.password)?;
        let restaurant_name = TenantName::new(command.restaurant_name)?;
        let owner_name = command
            .owner_name
            .map(DisplayName::new)
            .transpose()?;
        let language = command.language.unwrap_or_default();

        // Check if user already exists
        if self.user_repo.exists_by_email(&email).await? {
            return Err(AppError::conflict("User with this email already exists"));
        }

        // Create tenant
        let tenant = Tenant::new(restaurant_name);
        self.tenant_repo.create(&tenant).await?;

        // Hash password
        let password_hash = self.password_hasher.hash_password(password.as_str())?;

        // Create owner user
        let user = User::new(
            tenant.id,
            email,
            password_hash,
            owner_name,
            UserRole::Owner,
            language,
        );
        self.user_repo.create(&user).await?;

        // Generate tokens
        let access_token = self.jwt_service.generate_access_token(user.id, user.tenant_id)?;
        let refresh_token_str = self.jwt_service.generate_refresh_token();
        
        // Hash refresh token for storage using SHA256
        let mut hasher = Sha256::new();
        hasher.update(refresh_token_str.as_bytes());
        let refresh_token_hash = format!("{:x}", hasher.finalize());
        
        let expires_at = OffsetDateTime::now_utc() + self.jwt_service.get_refresh_token_ttl();
        
        let refresh_token = RefreshToken::new(user.id, refresh_token_hash, expires_at);
        self.refresh_token_repo.create(&refresh_token).await?;

        Ok(AuthResponse {
            access_token,
            refresh_token: refresh_token_str,
            user_id: user.id,
            tenant_id: user.tenant_id,
        })
    }

    pub async fn login(&self, command: LoginCommand) -> AppResult<AuthResponse> {
        // Validate input
        let email = Email::new(command.email)?;
        let password = Password::new(command.password)?;

        // Find user by email
        let user = self
            .user_repo
            .find_by_email(&email)
            .await?
            .ok_or_else(|| AppError::authentication("Invalid email or password"))?;

        // Verify password
        let password_valid = self
            .password_hasher
            .verify_password(password.as_str(), &user.password_hash)?;

        if !password_valid {
            return Err(AppError::authentication("Invalid email or password"));
        }

        // Generate tokens
        let access_token = self.jwt_service.generate_access_token(user.id, user.tenant_id)?;
        let refresh_token_str = self.jwt_service.generate_refresh_token();
        
        // Hash refresh token for storage using SHA256
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(refresh_token_str.as_bytes());
        let refresh_token_hash = format!("{:x}", hasher.finalize());
        
        let expires_at = OffsetDateTime::now_utc() + self.jwt_service.get_refresh_token_ttl();
        
        let refresh_token = RefreshToken::new(user.id, refresh_token_hash, expires_at);
        self.refresh_token_repo.create(&refresh_token).await?;

        Ok(AuthResponse {
            access_token,
            refresh_token: refresh_token_str,
            user_id: user.id,
            tenant_id: user.tenant_id,
        })
    }

    pub async fn refresh(&self, command: RefreshCommand) -> AppResult<AuthResponse> {
        // Hash the provided refresh token using SHA256 to match against stored hash
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(command.refresh_token.as_bytes());
        let token_hash = format!("{:x}", hasher.finalize());

        let stored_token = self
            .refresh_token_repo
            .find_by_token_hash(&token_hash)
            .await?
            .ok_or_else(|| AppError::authentication("Invalid refresh token"))?;

        // Validate token
        if !stored_token.is_valid() {
            return Err(AppError::authentication("Refresh token expired or revoked"));
        }

        // Get user
        let user = self
            .user_repo
            .find_by_id(stored_token.user_id)
            .await?
            .ok_or_else(|| AppError::authentication("User not found"))?;

        // Generate new access token
        let access_token = self.jwt_service.generate_access_token(user.id, user.tenant_id)?;

        Ok(AuthResponse {
            access_token,
            refresh_token: command.refresh_token, // Return the same refresh token
            user_id: user.id,
            tenant_id: user.tenant_id,
        })
    }
}

#[derive(Debug, Deserialize)]
pub struct RegisterCommand {
    pub email: String,
    pub password: String,
    pub restaurant_name: String,
    pub owner_name: Option<String>,
    pub language: Option<Language>,
}

#[derive(Debug, Deserialize)]
pub struct LoginCommand {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct RefreshCommand {
    pub refresh_token: String,
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub user_id: UserId,
    pub tenant_id: TenantId,
}
