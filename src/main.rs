use lock::Lock;
use std::sync::Arc;

use axum::{
	extract::{self, Path},
	http::StatusCode,
	response::IntoResponse,
	routing::post,
	Json, Router,
};

use dashmap::DashMap;

mod lock;

#[derive(Clone)]
pub struct State {
	pub(crate) locks: Arc<DashMap<String, Lock>>,
}

impl State {
	pub fn new() -> Self {
		Self::new_with_data(Arc::new(DashMap::new()))
	}

	pub fn new_with_data(data: Arc<DashMap<String, Lock>>) -> Self {
		Self { locks: data }
	}
}

#[derive(Debug)]
pub enum Error {
	NotFound,
}

impl IntoResponse for Error {
	fn into_response(self) -> axum::response::Response {
		let status = match self {
			Error::NotFound => StatusCode::GONE,
		};

		status.into_response()
	}
}

#[tokio::main]
async fn main() -> Result<(), Error> {
	let addr: std::net::SocketAddr = std::net::SocketAddr::from(([0, 0, 0, 0], 3000));

	axum::Server::bind(&addr)
		.serve(router(State::new()).into_make_service())
		.await
		.unwrap();

	Ok(())
}

#[allow(dead_code)]
fn router(state: State) -> Router {
	Router::new()
		.route("/lock/:id", post(lock))
		.route("/unlock/:id", post(unlock))
		.route("/purge", post(purge))
		.with_state(state)
}

pub async fn lock(
	extract::State(state): extract::State<State>,
	Path(id): Path<String>,
	extract::Json(lock): extract::Json<Lock>,
) -> Result<StatusCode, Error> {
	state.locks.insert(id.clone(), lock.clone());

	Ok(StatusCode::CREATED)
}

pub async fn unlock(
	extract::State(state): extract::State<State>,
	Path(id): Path<String>,
) -> Result<(StatusCode, Json<Lock>), Error> {
	if let Some((_, lock)) = state.locks.remove(&id) {
		Ok((StatusCode::OK, Json(lock)))
	} else {
		Err(Error::NotFound)
	}
}

pub async fn purge(extract::State(state): extract::State<State>) -> Result<StatusCode, Error> {
	state.locks.clear();

	Ok(StatusCode::OK)
}
