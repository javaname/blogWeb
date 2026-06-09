use axum::{
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::Row;

use crate::{
    admin_auth::{auth_required, session_user},
    error::{AppError, Result},
    http_public::PublicState,
    session::SessionUser,
};

pub const PERMISSION_PUBLISH: &str = "publish";
pub const PERMISSION_MODERATE: &str = "moderate";
pub const PERMISSION_SETTINGS: &str = "settings";
pub const PERMISSION_USERS: &str = "users";
pub const PERMISSION_MCP: &str = "mcp";
pub const PERMISSION_MEDIA: &str = "media";
pub const PERMISSION_ANALYTICS: &str = "analytics";

const ROLE_ADMIN: &str = "admin";
const ROLE_EDITOR: &str = "editor";
const ROLE_WRITER: &str = "writer";
const ROLE_USER: &str = "user";

#[derive(Debug, Deserialize)]
pub struct UpdateRolePermissionsRequest {
    pub roles: Vec<RolePermissionsInput>,
}

#[derive(Debug, Deserialize)]
pub struct RolePermissionsInput {
    pub key: String,
    pub permissions: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct PermissionDefinition {
    key: &'static str,
    label: &'static str,
    description: &'static str,
}

#[derive(Debug, Serialize)]
pub struct RoleDefinition {
    key: &'static str,
    label: &'static str,
    description: &'static str,
    permissions: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct MenuDefinition {
    path: &'static str,
    label_key: &'static str,
    icon: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    permission: Option<&'static str>,
}

pub async fn require_permission(
    state: &PublicState,
    headers: &HeaderMap,
    permission: &str,
) -> std::result::Result<SessionUser, Response> {
    let Some(user) = session_user(state, headers).await else {
        return Err(auth_required());
    };
    let allowed = match role_has_permission(state, &user.role, permission).await {
        Ok(allowed) => allowed,
        Err(_) => {
            return Err(json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal_error",
                "权限检查失败",
            ));
        }
    };
    if allowed {
        Ok(user)
    } else {
        Err(forbidden())
    }
}

pub async fn require_permission_csrf(
    state: &PublicState,
    headers: &HeaderMap,
    permission: &str,
) -> std::result::Result<SessionUser, Response> {
    let user = require_permission(state, headers, permission).await?;
    let token = headers
        .get("x-csrf-token")
        .and_then(|value| value.to_str().ok());
    if token == Some(user.csrf_token.as_str()) {
        Ok(user)
    } else {
        Err(json_error(
            StatusCode::FORBIDDEN,
            "csrf_invalid",
            "CSRF token 无效",
        ))
    }
}

pub async fn role_has_permission(
    state: &PublicState,
    role: &str,
    permission: &str,
) -> Result<bool> {
    Ok(role_permissions(state, role)
        .await?
        .iter()
        .any(|value| value == permission))
}

pub async fn role_permissions(state: &PublicState, role: &str) -> Result<Vec<String>> {
    let role = normalize_role(role)?;
    let rows = sqlx::query(crate::db::sql(
        "SELECT permission
         FROM role_permissions
         WHERE role = ?
         ORDER BY permission ASC",
    ))
    .bind(role)
    .fetch_all(&state.db)
    .await?;
    if rows.is_empty() {
        return Ok(default_role_permissions(role)
            .iter()
            .map(|value| (*value).to_string())
            .collect());
    }
    let mut permissions = rows
        .into_iter()
        .map(|row| row.try_get::<String, _>("permission"))
        .collect::<std::result::Result<Vec<_>, _>>()?;
    sort_permissions(&mut permissions);
    Ok(permissions)
}

pub async fn role_definitions(state: &PublicState) -> Result<Vec<RoleDefinition>> {
    let mut roles = Vec::new();
    for role in role_metadata() {
        roles.push(RoleDefinition {
            key: role.key,
            label: role.label,
            description: role.description,
            permissions: role_permissions(state, role.key).await?,
        });
    }
    Ok(roles)
}

pub fn permission_definitions() -> Vec<PermissionDefinition> {
    vec![
        PermissionDefinition {
            key: PERMISSION_PUBLISH,
            label: "内容发布",
            description: "创建、编辑和发布文章及分类",
        },
        PermissionDefinition {
            key: PERMISSION_MODERATE,
            label: "评论审核",
            description: "处理评论状态和删除违规内容",
        },
        PermissionDefinition {
            key: PERMISSION_SETTINGS,
            label: "系统设置",
            description: "更新站点配置和运行策略",
        },
        PermissionDefinition {
            key: PERMISSION_USERS,
            label: "用户管理",
            description: "新增用户、调整成员角色和配置角色权限",
        },
        PermissionDefinition {
            key: PERMISSION_MCP,
            label: "MCP 接入",
            description: "管理外部客户端和发布能力",
        },
        PermissionDefinition {
            key: PERMISSION_MEDIA,
            label: "媒体库",
            description: "上传和管理站内图片素材",
        },
        PermissionDefinition {
            key: PERMISSION_ANALYTICS,
            label: "数据分析",
            description: "查看阅读趋势和内容表现",
        },
    ]
}

pub async fn menus_for_role(state: &PublicState, role: &str) -> Result<Vec<MenuDefinition>> {
    let permissions = role_permissions(state, role).await?;
    Ok(menu_definitions()
        .into_iter()
        .filter(|menu| {
            menu.permission
                .is_none_or(|permission| permissions.iter().any(|value| value == permission))
        })
        .collect())
}

pub fn menu_definitions() -> Vec<MenuDefinition> {
    vec![
        MenuDefinition {
            path: "/dashboard",
            label_key: "shell.navDashboard",
            icon: "dashboard",
            permission: None,
        },
        MenuDefinition {
            path: "/posts",
            label_key: "shell.navPosts",
            icon: "article",
            permission: Some(PERMISSION_PUBLISH),
        },
        MenuDefinition {
            path: "/categories",
            label_key: "shell.navCategories",
            icon: "category",
            permission: Some(PERMISSION_PUBLISH),
        },
        MenuDefinition {
            path: "/comments",
            label_key: "shell.navComments",
            icon: "comment",
            permission: Some(PERMISSION_MODERATE),
        },
        MenuDefinition {
            path: "/media",
            label_key: "shell.navMedia",
            icon: "image",
            permission: Some(PERMISSION_MEDIA),
        },
        MenuDefinition {
            path: "/users",
            label_key: "shell.navUsers",
            icon: "group",
            permission: Some(PERMISSION_USERS),
        },
        MenuDefinition {
            path: "/roles",
            label_key: "shell.navRoles",
            icon: "tune",
            permission: Some(PERMISSION_USERS),
        },
        MenuDefinition {
            path: "/analytics",
            label_key: "shell.navAnalytics",
            icon: "trending_up",
            permission: Some(PERMISSION_ANALYTICS),
        },
        MenuDefinition {
            path: "/settings",
            label_key: "shell.navSettings",
            icon: "settings",
            permission: Some(PERMISSION_SETTINGS),
        },
    ]
}

pub async fn replace_role_permissions(
    state: &PublicState,
    request: UpdateRolePermissionsRequest,
    current_user: &SessionUser,
) -> Result<std::result::Result<Vec<RoleDefinition>, Response>> {
    let mut normalized = Vec::new();
    for item in request.roles {
        let role = normalize_role(&item.key)?;
        let mut permissions = Vec::new();
        for permission in item.permissions {
            let permission = normalize_permission(&permission)?;
            if !permissions.iter().any(|value| value == permission) {
                permissions.push(permission.to_string());
            }
        }
        sort_permissions(&mut permissions);
        normalized.push((role.to_string(), permissions));
    }

    if !normalized.iter().any(|(role, _)| role == ROLE_ADMIN) {
        normalized.push((
            ROLE_ADMIN.to_string(),
            default_role_permissions(ROLE_ADMIN)
                .iter()
                .map(|value| (*value).to_string())
                .collect(),
        ));
    }

    let current_role_permissions = normalized
        .iter()
        .find(|(role, _)| role == current_user.role.as_str())
        .map(|(_, permissions)| permissions.as_slice())
        .unwrap_or_default();
    if !current_role_permissions
        .iter()
        .any(|permission| permission == PERMISSION_USERS)
    {
        return Ok(Err(json_error(
            StatusCode::BAD_REQUEST,
            "invalid_params",
            "不能移除当前角色的用户管理权限",
        )));
    }

    let mut transaction = state.db.begin().await?;
    sqlx::query("DELETE FROM role_permissions")
        .execute(&mut *transaction)
        .await?;
    for (role, permissions) in &normalized {
        for permission in permissions {
            sqlx::query(crate::db::sql(
                "INSERT INTO role_permissions (role, permission, created_at)
                 VALUES (?, ?, CURRENT_TIMESTAMP::text)
                 ON CONFLICT(role, permission) DO NOTHING",
            ))
            .bind(role)
            .bind(permission)
            .execute(&mut *transaction)
            .await?;
        }
    }
    transaction.commit().await?;
    Ok(Ok(role_definitions(state).await?))
}

pub fn normalize_role(role: &str) -> Result<&'static str> {
    match role.trim() {
        ROLE_ADMIN => Ok(ROLE_ADMIN),
        ROLE_EDITOR => Ok(ROLE_EDITOR),
        ROLE_WRITER => Ok(ROLE_WRITER),
        ROLE_USER => Ok(ROLE_USER),
        _ => Err(AppError::HttpStatus(400, "invalid_params".into())),
    }
}

fn normalize_permission(permission: &str) -> Result<&'static str> {
    match permission.trim() {
        PERMISSION_PUBLISH => Ok(PERMISSION_PUBLISH),
        PERMISSION_MODERATE => Ok(PERMISSION_MODERATE),
        PERMISSION_SETTINGS => Ok(PERMISSION_SETTINGS),
        PERMISSION_USERS => Ok(PERMISSION_USERS),
        PERMISSION_MCP => Ok(PERMISSION_MCP),
        PERMISSION_MEDIA => Ok(PERMISSION_MEDIA),
        PERMISSION_ANALYTICS => Ok(PERMISSION_ANALYTICS),
        _ => Err(AppError::HttpStatus(400, "invalid_params".into())),
    }
}

fn default_role_permissions(role: &str) -> &'static [&'static str] {
    match role {
        ROLE_ADMIN => &[
            PERMISSION_PUBLISH,
            PERMISSION_MODERATE,
            PERMISSION_SETTINGS,
            PERMISSION_USERS,
            PERMISSION_MCP,
            PERMISSION_MEDIA,
            PERMISSION_ANALYTICS,
        ],
        ROLE_EDITOR => &[PERMISSION_PUBLISH, PERMISSION_MODERATE],
        ROLE_WRITER => &[PERMISSION_PUBLISH],
        _ => &[],
    }
}

fn sort_permissions(permissions: &mut [String]) {
    let order = permission_definitions()
        .into_iter()
        .map(|item| item.key)
        .collect::<Vec<_>>();
    permissions.sort_by_key(|permission| {
        order
            .iter()
            .position(|value| value == permission)
            .unwrap_or(usize::MAX)
    });
}

struct RoleMetadata {
    key: &'static str,
    label: &'static str,
    description: &'static str,
}

fn role_metadata() -> Vec<RoleMetadata> {
    vec![
        RoleMetadata {
            key: ROLE_ADMIN,
            label: "管理员",
            description: "拥有全部后台权限",
        },
        RoleMetadata {
            key: ROLE_EDITOR,
            label: "编辑",
            description: "管理内容发布和评论审核",
        },
        RoleMetadata {
            key: ROLE_WRITER,
            label: "作者",
            description: "创建和维护自己的内容",
        },
        RoleMetadata {
            key: ROLE_USER,
            label: "普通用户",
            description: "仅保留前台读者身份",
        },
    ]
}

pub fn forbidden() -> Response {
    json_error(StatusCode::FORBIDDEN, "forbidden", "权限不足")
}

pub fn json_error(status: StatusCode, code: &str, message: &str) -> Response {
    (
        status,
        Json(json!({
            "code": code,
            "message": message,
        })),
    )
        .into_response()
}
