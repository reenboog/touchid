use std::{collections::BTreeMap, sync::Arc};
use lock::Lock;

use axum::{
	extract::{self, Path},
	http::StatusCode,
	response::IntoResponse,
	routing::post,
	Json, Router,
};

use tokio::sync::Mutex;

mod lock;

#[derive(Clone)]
pub struct State {
	pub(crate) users: Arc<Mutex<BTreeMap<String, Lock>>>,
}

impl State {
	pub fn new() -> Self {
		Self::new_with_data(Arc::new(Mutex::new(BTreeMap::new())))
	}

	pub fn new_with_data(data: Arc<Mutex<BTreeMap<String, Lock>>>) -> Self {
		Self { users: data }
	}
}

#[derive(Debug)]
pub enum Error {
	NotFound,
}

impl IntoResponse for Error {
	fn into_response(self) -> axum::response::Response {
		let status = match self {
			Error::NotFound => StatusCode::GONE
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
	let mut locks = state.users.lock().await;

	locks.insert(id.clone(), lock.clone());

	Ok(StatusCode::CREATED)
}

pub async fn unlock(
	extract::State(state): extract::State<State>,
	Path(id): Path<String>,
) -> Result<(StatusCode, Json<Lock>), Error> {
	let mut locks = state.users.lock().await;

	if let Some(lock) = locks.remove(&id) {
		Ok((StatusCode::OK, Json(lock)))
	} else {
		Err(Error::NotFound)
	}
}

pub async fn purge(
	extract::State(state): extract::State<State>,
) -> Result<StatusCode, Error> {
	let mut locks = state.users.lock().await;

	locks.clear();

	Ok(StatusCode::OK)
}