use crate::{util, AppState, AuthContext};
use api::ApiErrorCode;
use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Extension, Router,
};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use time::OffsetDateTime;
use tracing::Span;
use uuid::Uuid;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/orgs/{org_id}/friends", get(list_friends))
        .route(
            "/orgs/{org_id}/friends/requests",
            get(list_requests).post(create_request),
        )
        .route(
            "/orgs/{org_id}/friends/requests/{request_id}/accept",
            post(accept_request),
        )
        .route(
            "/orgs/{org_id}/friends/requests/{request_id}/decline",
            post(decline_request),
        )
        .route(
            "/orgs/{org_id}/friends/requests/{request_id}/cancel",
            post(cancel_request),
        )
        .route(
            "/orgs/{org_id}/friends/{user_id}",
            axum::routing::delete(remove_friend),
        )
}

#[derive(Debug, Serialize)]
struct UserSummary {
    id: Uuid,
    email: String,
    display_name: String,
}

#[derive(Debug, Serialize)]
struct FriendRequestResponse {
    id: Uuid,
    requester: UserSummary,
    addressee: UserSummary,
    status: String,
    created_at: OffsetDateTime,
    responded_at: Option<OffsetDateTime>,
}

#[derive(Debug, Serialize)]
struct FriendRequestsResponse {
    requests: Vec<FriendRequestResponse>,
}

#[derive(Debug, Serialize)]
struct FriendsResponse {
    friends: Vec<UserSummary>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
struct CreateFriendRequestRequest {
    user_id: Uuid,
}

async fn list_requests(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(org_id): Path<Uuid>,
) -> impl IntoResponse {
    Span::current().record("organization_id", tracing::field::display(org_id));
    let is_member = match util::is_member(&state.pool, org_id, auth.user_id).await {
        Ok(v) => v,
        Err(e) => return e,
    };
    if !is_member {
        return util::api_error(ApiErrorCode::PermissionDenied);
    }

    let rows = sqlx::query(
        r#"
        select
          fr.id,
          fr.requester_id,
          fr.addressee_id,
          fr.status,
          fr.created_at,
          fr.responded_at,
          ru.email as requester_email,
          ru.display_name as requester_display_name,
          au.email as addressee_email,
          au.display_name as addressee_display_name
        from friend_requests fr
        join users ru on ru.id = fr.requester_id
        join users au on au.id = fr.addressee_id
        where fr.organization_id = $1
          and fr.status = 'pending'
          and (fr.requester_id = $2 or fr.addressee_id = $2)
        order by fr.created_at desc
        "#,
    )
    .bind(org_id)
    .bind(auth.user_id)
    .fetch_all(&state.pool)
    .await;

    let rows = match rows {
        Ok(r) => r,
        Err(_) => return util::api_error(ApiErrorCode::InternalError),
    };

    let mut requests = Vec::with_capacity(rows.len());
    for r in rows.iter() {
        requests.push(FriendRequestResponse {
            id: r.get("id"),
            requester: UserSummary {
                id: r.get("requester_id"),
                email: r.get("requester_email"),
                display_name: r.get("requester_display_name"),
            },
            addressee: UserSummary {
                id: r.get("addressee_id"),
                email: r.get("addressee_email"),
                display_name: r.get("addressee_display_name"),
            },
            status: r.get("status"),
            created_at: r.get("created_at"),
            responded_at: r.get("responded_at"),
        });
    }

    (StatusCode::OK, Json(FriendRequestsResponse { requests })).into_response()
}

async fn list_friends(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(org_id): Path<Uuid>,
) -> impl IntoResponse {
    Span::current().record("organization_id", tracing::field::display(org_id));
    let is_member = match util::is_member(&state.pool, org_id, auth.user_id).await {
        Ok(v) => v,
        Err(e) => return e,
    };
    if !is_member {
        return util::api_error(ApiErrorCode::PermissionDenied);
    }

    let rows = sqlx::query(
        r#"
        select
          u.id as other_id,
          u.email as other_email,
          u.display_name as other_display_name
        from friend_requests fr
        join users u
          on u.id = case
            when fr.requester_id = $2 then fr.addressee_id
            else fr.requester_id
          end
        where fr.organization_id = $1
          and fr.status = 'accepted'
          and (fr.requester_id = $2 or fr.addressee_id = $2)
        order by u.display_name asc
        "#,
    )
    .bind(org_id)
    .bind(auth.user_id)
    .fetch_all(&state.pool)
    .await;

    let rows = match rows {
        Ok(r) => r,
        Err(_) => return util::api_error(ApiErrorCode::InternalError),
    };

    let mut friends = Vec::with_capacity(rows.len());
    for r in rows.iter() {
        friends.push(UserSummary {
            id: r.get("other_id"),
            email: r.get("other_email"),
            display_name: r.get("other_display_name"),
        });
    }

    (StatusCode::OK, Json(FriendsResponse { friends })).into_response()
}

async fn create_request(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(org_id): Path<Uuid>,
    Json(req): Json<CreateFriendRequestRequest>,
) -> impl IntoResponse {
    Span::current().record("organization_id", tracing::field::display(org_id));
    if req.user_id == auth.user_id {
        return util::api_error(ApiErrorCode::ValidationError);
    }

    let me_member = match util::is_member(&state.pool, org_id, auth.user_id).await {
        Ok(v) => v,
        Err(e) => return e,
    };
    let them_member = match util::is_member(&state.pool, org_id, req.user_id).await {
        Ok(v) => v,
        Err(e) => return e,
    };
    if !me_member || !them_member {
        return util::api_error(ApiErrorCode::PermissionDenied);
    }

    // If the other user already requested me, accept it.
    let now = OffsetDateTime::now_utc();
    let flipped = sqlx::query(
        r#"
        update friend_requests
        set status = 'accepted', responded_at = $4
        where organization_id = $1
          and requester_id = $2
          and addressee_id = $3
          and status = 'pending'
        returning id
        "#,
    )
    .bind(org_id)
    .bind(req.user_id)
    .bind(auth.user_id)
    .bind(now)
    .fetch_optional(&state.pool)
    .await;

    if let Ok(Some(row)) = flipped {
        let request_id: Uuid = row.get("id");
        return (
            StatusCode::OK,
            Json(serde_json::json!({ "status": "accepted", "request_id": request_id })),
        )
            .into_response();
    }

    let request_id = Uuid::now_v7();
    let inserted = sqlx::query(
        r#"
        insert into friend_requests (id, organization_id, requester_id, addressee_id, status, created_at)
        values ($1, $2, $3, $4, 'pending', $5)
        "#,
    )
    .bind(request_id)
    .bind(org_id)
    .bind(auth.user_id)
    .bind(req.user_id)
    .bind(now)
    .execute(&state.pool)
    .await;

    match inserted {
        Ok(_) => (
            StatusCode::OK,
            Json(serde_json::json!({ "status": "pending", "request_id": request_id })),
        )
            .into_response(),
        Err(err) => {
            if util::is_unique_violation(&err) {
                let existing = sqlx::query_scalar::<_, Uuid>(
                    r#"
                    select id
                    from friend_requests
                    where organization_id = $1
                      and requester_id = $2
                      and addressee_id = $3
                      and status = 'pending'
                    order by created_at desc
                    limit 1
                    "#,
                )
                .bind(org_id)
                .bind(auth.user_id)
                .bind(req.user_id)
                .fetch_optional(&state.pool)
                .await;

                let request_id = match existing {
                    Ok(Some(id)) => id,
                    Ok(None) => return util::api_error(ApiErrorCode::Conflict),
                    Err(_) => return util::api_error(ApiErrorCode::InternalError),
                };

                return (
                    StatusCode::OK,
                    Json(serde_json::json!({ "status": "pending", "request_id": request_id })),
                )
                    .into_response();
            }
            util::api_error(ApiErrorCode::InternalError)
        }
    }
}

async fn accept_request(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path((org_id, request_id)): Path<(Uuid, Uuid)>,
) -> impl IntoResponse {
    Span::current().record("organization_id", tracing::field::display(org_id));
    let is_member = match util::is_member(&state.pool, org_id, auth.user_id).await {
        Ok(v) => v,
        Err(e) => return e,
    };
    if !is_member {
        return util::api_error(ApiErrorCode::PermissionDenied);
    }

    let now = OffsetDateTime::now_utc();
    let updated = sqlx::query(
        r#"
        update friend_requests
        set status = 'accepted', responded_at = $4
        where id = $1
          and organization_id = $2
          and addressee_id = $3
          and status = 'pending'
        "#,
    )
    .bind(request_id)
    .bind(org_id)
    .bind(auth.user_id)
    .bind(now)
    .execute(&state.pool)
    .await;

    match updated {
        Ok(r) if r.rows_affected() == 1 => {
            (StatusCode::OK, Json(serde_json::json!({ "status": "accepted" }))).into_response()
        }
        Ok(_) => util::api_error(ApiErrorCode::NotFound),
        Err(_) => util::api_error(ApiErrorCode::InternalError),
    }
}

async fn decline_request(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path((org_id, request_id)): Path<(Uuid, Uuid)>,
) -> impl IntoResponse {
    Span::current().record("organization_id", tracing::field::display(org_id));
    let is_member = match util::is_member(&state.pool, org_id, auth.user_id).await {
        Ok(v) => v,
        Err(e) => return e,
    };
    if !is_member {
        return util::api_error(ApiErrorCode::PermissionDenied);
    }

    let now = OffsetDateTime::now_utc();
    let updated = sqlx::query(
        r#"
        update friend_requests
        set status = 'declined', responded_at = $4
        where id = $1
          and organization_id = $2
          and addressee_id = $3
          and status = 'pending'
        "#,
    )
    .bind(request_id)
    .bind(org_id)
    .bind(auth.user_id)
    .bind(now)
    .execute(&state.pool)
    .await;

    match updated {
        Ok(r) if r.rows_affected() == 1 => {
            (StatusCode::OK, Json(serde_json::json!({ "status": "declined" }))).into_response()
        }
        Ok(_) => util::api_error(ApiErrorCode::NotFound),
        Err(_) => util::api_error(ApiErrorCode::InternalError),
    }
}

async fn cancel_request(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path((org_id, request_id)): Path<(Uuid, Uuid)>,
) -> impl IntoResponse {
    Span::current().record("organization_id", tracing::field::display(org_id));
    let is_member = match util::is_member(&state.pool, org_id, auth.user_id).await {
        Ok(v) => v,
        Err(e) => return e,
    };
    if !is_member {
        return util::api_error(ApiErrorCode::PermissionDenied);
    }

    let now = OffsetDateTime::now_utc();
    let updated = sqlx::query(
        r#"
        update friend_requests
        set status = 'cancelled', responded_at = $4
        where id = $1
          and organization_id = $2
          and requester_id = $3
          and status = 'pending'
        "#,
    )
    .bind(request_id)
    .bind(org_id)
    .bind(auth.user_id)
    .bind(now)
    .execute(&state.pool)
    .await;

    match updated {
        Ok(r) if r.rows_affected() == 1 => {
            (StatusCode::OK, Json(serde_json::json!({ "status": "cancelled" }))).into_response()
        }
        Ok(_) => util::api_error(ApiErrorCode::NotFound),
        Err(_) => util::api_error(ApiErrorCode::InternalError),
    }
}

async fn remove_friend(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path((org_id, user_id)): Path<(Uuid, Uuid)>,
) -> impl IntoResponse {
    Span::current().record("organization_id", tracing::field::display(org_id));
    if user_id == auth.user_id {
        return util::api_error(ApiErrorCode::ValidationError);
    }

    let is_member = match util::is_member(&state.pool, org_id, auth.user_id).await {
        Ok(v) => v,
        Err(e) => return e,
    };
    if !is_member {
        return util::api_error(ApiErrorCode::PermissionDenied);
    }

    let now = OffsetDateTime::now_utc();
    let mut tx = match state.pool.begin().await {
        Ok(tx) => tx,
        Err(_) => return util::api_error(ApiErrorCode::InternalError),
    };

    let updated = sqlx::query(
        r#"
        update friend_requests
        set status = 'cancelled', responded_at = $4
        where organization_id = $1
          and status = 'accepted'
          and (
            (requester_id = $2 and addressee_id = $3)
            or
            (requester_id = $3 and addressee_id = $2)
          )
        "#,
    )
    .bind(org_id)
    .bind(auth.user_id)
    .bind(user_id)
    .bind(now)
    .execute(&mut *tx)
    .await;

    let updated = match updated {
        Ok(r) => r.rows_affected(),
        Err(_) => return util::api_error(ApiErrorCode::InternalError),
    };
    if updated == 0 {
        return util::api_error(ApiErrorCode::NotFound);
    }

    // Also delete any existing DM channel between the two users in this org.
    let dm_channel = sqlx::query_scalar::<_, Uuid>(
        r#"
        select c.id
        from channels c
        join dm_channel_members a on a.channel_id = c.id and a.user_id = $2
        join dm_channel_members b on b.channel_id = c.id and b.user_id = $3
        where c.organization_id = $1 and c.kind = 'dm'
        limit 1
        "#,
    )
    .bind(org_id)
    .bind(auth.user_id)
    .bind(user_id)
    .fetch_optional(&mut *tx)
    .await;

    if let Ok(Some(channel_id)) = dm_channel {
        let _ = sqlx::query(r#"delete from channels where id = $1"#)
            .bind(channel_id)
            .execute(&mut *tx)
            .await;
    }

    if tx.commit().await.is_err() {
        return util::api_error(ApiErrorCode::InternalError);
    }

    (StatusCode::OK, Json(serde_json::json!({ "status": "removed" }))).into_response()
}

