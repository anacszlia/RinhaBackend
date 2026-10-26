mod lua;
mod validar;

use std::time::Duration;

use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, patch, post};
use axum::Router;
use redis::aio::ConnectionManager;

#[derive(Clone)]
struct App {
    redis: ConnectionManager,
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://redis:6379".to_string());
    let porta = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let redis = conectar(&url).await;
    let app = Router::new()
        .route("/saude", get(saude))
        .route("/placar", get(placar))
        .route("/admin/reset", post(reset))
        .route("/equipes", post(criar_equipe))
        .route("/equipes/{*id}", get(detalhe_equipe))
        .route("/submissoes", post(criar_submissao))
        .route("/submissoes/{*id}", patch(rejulgar))
        .with_state(App { redis });

    let addr = format!("0.0.0.0:{porta}");
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .unwrap_or_else(|erro| panic!("bind {addr}: {erro}"));
    eprintln!("ouvindo {addr}");
    axum::serve(listener, app).await.unwrap();
}

async fn conectar(url: &str) -> ConnectionManager {
    let cliente = redis::Client::open(url).unwrap_or_else(|erro| panic!("redis url: {erro}"));
    loop {
        if let Ok(manager) = ConnectionManager::new(cliente.clone()).await {
            let mut conn = manager.clone();
            if let Ok(pong) = redis::cmd("PING").query_async::<String>(&mut conn).await {
                if pong == "PONG" {
                    return manager;
                }
            }
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
}

async fn saude(State(app): State<App>) -> StatusCode {
    let mut conn = app.redis.clone();
    match redis::cmd("PING").query_async::<String>(&mut conn).await {
        Ok(pong) if pong == "PONG" => StatusCode::OK,
        _ => StatusCode::SERVICE_UNAVAILABLE,
    }
}

async fn placar(State(app): State<App>) -> Response {
    let mut conn = app.redis.clone();
    match redis::cmd("GET").arg("placar").query_async::<Option<String>>(&mut conn).await {
        Ok(Some(json)) => json_ok(json),
        Ok(None) => json_ok("[]".to_string()),
        Err(erro) => erro_redis(erro),
    }
}

async fn detalhe_equipe(State(app): State<App>, Path(id): Path<String>) -> Response {
    let mut conn = app.redis.clone();
    let chave = format!("detalhe:{id}");
    match redis::cmd("GET").arg(chave).query_async::<Option<String>>(&mut conn).await {
        Ok(Some(json)) => json_ok(json),
        Ok(None) => vazio(StatusCode::NOT_FOUND),
        Err(erro) => erro_redis(erro),
    }
}

async fn criar_equipe(State(app): State<App>, corpo: Bytes) -> Response {
    let Ok(dados) = validar::equipe(&corpo) else {
        return vazio(StatusCode::UNPROCESSABLE_ENTITY);
    };
    let mut conn = app.redis.clone();
    match lua::EQUIPE.arg(&dados.id).arg(&dados.nome).invoke_async::<i64>(&mut conn).await {
        Ok(codigo) => vazio(status_de(codigo)),
        Err(erro) => erro_redis(erro),
    }
}

async fn criar_submissao(State(app): State<App>, corpo: Bytes) -> Response {
    let Ok(dados) = validar::submissao(&corpo) else {
        return vazio(StatusCode::UNPROCESSABLE_ENTITY);
    };
    let mut conn = app.redis.clone();
    match lua::SUBMISSAO
        .arg(&dados.id)
        .arg(&dados.equipe_id)
        .arg(&dados.problema)
        .arg(dados.minuto)
        .arg(&dados.veredito)
        .invoke_async::<i64>(&mut conn)
        .await
    {
        Ok(codigo) => vazio(status_de(codigo)),
        Err(erro) => erro_redis(erro),
    }
}

async fn rejulgar(State(app): State<App>, Path(id): Path<String>, corpo: Bytes) -> Response {
    let Ok(veredito) = validar::veredito(&corpo) else {
        return vazio(StatusCode::UNPROCESSABLE_ENTITY);
    };
    let mut conn = app.redis.clone();
    match lua::REJULGAR.arg(&id).arg(&veredito).invoke_async::<i64>(&mut conn).await {
        Ok(codigo) => vazio(status_de(codigo)),
        Err(erro) => erro_redis(erro),
    }
}

async fn reset(State(app): State<App>) -> Response {
    // FLUSHDB tem a flag noscript no Redis, então não entra no Lua.
    let mut conn = app.redis.clone();
    match redis::cmd("FLUSHDB").query_async::<()>(&mut conn).await {
        Ok(()) => vazio(StatusCode::NO_CONTENT),
        Err(erro) => erro_redis(erro),
    }
}

fn status_de(codigo: i64) -> StatusCode {
    StatusCode::from_u16(codigo as u16).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
}

fn vazio(codigo: StatusCode) -> Response {
    codigo.into_response()
}

fn json_ok(corpo: String) -> Response {
    (StatusCode::OK, [(header::CONTENT_TYPE, "application/json")], corpo).into_response()
}

fn erro_redis(erro: redis::RedisError) -> Response {
    eprintln!("redis: {erro}");
    vazio(StatusCode::INTERNAL_SERVER_ERROR)
}
