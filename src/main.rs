use serde_json::json;
use std::{collections::BTreeMap, sync::Arc};
use user::User;

use axum::{
	extract::{self, Path},
	http::StatusCode,
	response::IntoResponse,
	routing::{delete, get, post},
	Json, Router,
};

use tokio::sync::Mutex;

mod user;

#[derive(Clone)]
pub struct State {
	pub(crate) users: Arc<Mutex<BTreeMap<i32, User>>>,
}

impl State {
	pub fn new() -> Self {
		Self::new_with_data(Arc::new(Mutex::new(BTreeMap::new())))
	}

	pub fn new_with_data(data: Arc<Mutex<BTreeMap<i32, User>>>) -> Self {
		Self { users: data }
	}
}

#[derive(Debug)]
pub enum Error {
	NotFound,
	AlreadyExists,
}

impl IntoResponse for Error {
	fn into_response(self) -> axum::response::Response {
		let (status, err) = match self {
			Error::NotFound => (StatusCode::NOT_FOUND, "not found"),
			Error::AlreadyExists => (StatusCode::CONFLICT, "already exists"),
		};

		let body = Json(json!({
					"error": err,
		}));

		(status, body).into_response()
	}
}

#[tokio::main]
async fn main() -> Result<(), Error> {
	let addr: std::net::SocketAddr = std::net::SocketAddr::from(([0, 0, 0, 0], 3000));

	println!("quku api listening on {}", addr);

	axum::Server::bind(&addr)
		.serve(router(State::new()).into_make_service())
		.await
		.unwrap();

	Ok(())
}

#[allow(dead_code)]
fn router(state: State) -> Router {
	Router::new()
		.route("/users", get(get_users))
		.route("/users/:id", get(get_user))
		.route("/users", post(create_user))
		.route("/users/:id", delete(delete_user))
		.with_state(state)
}

async fn get_users(extract::State(state): extract::State<State>) -> Json<Vec<User>> {
	let users = state.users.lock().await;

	Json(users.values().cloned().collect())
}

async fn get_user(
	extract::State(state): extract::State<State>,
	Path(user_id): Path<i32>,
) -> Result<Json<User>, Error> {
	let users = state.users.lock().await;
	let user = users.get(&user_id).ok_or(Error::NotFound)?.clone();

	Ok(user.into())
}

// FIXME: this might be unsafe on prod since it reveals who's using the service
pub async fn create_user(
	extract::State(state): extract::State<State>,
	extract::Json(user): extract::Json<User>,
) -> Result<(StatusCode, Json<User>), Error> {
	let mut users = state.users.lock().await;

	if users.contains_key(&user.id) {
		Err(Error::AlreadyExists)
	} else {
		users.insert(user.id, user.clone());

		Ok((StatusCode::CREATED, Json(user)))
	}
}

// FIXME: returning 204 | 409 might be unsafe on prod
pub async fn delete_user(
	extract::State(state): extract::State<State>,
	Path(user_id): Path<i32>,
) -> Result<StatusCode, Error> {
	let mut users = state.users.lock().await;

	if let Some(_) = users.remove(&user_id) {
		Ok(StatusCode::NO_CONTENT)
	} else {
		Err(Error::NotFound)
	}
}

#[cfg(test)]
mod tests {
	use crate::{router, State, User};
	use axum::{body::Body, http, Router};
	use hyper::{self, Request};
	use serde_json::{self};
	use std::{collections::BTreeMap, sync::Arc};
	use tokio::sync::Mutex;
	use tower::{util::Oneshot, ServiceExt};

	fn call(
		router: Router,
		uri: &str,
		method: http::Method,
		body: Body,
		mime: mime::Mime,
	) -> Oneshot<Router, Request<Body>> {
		router.oneshot(
			http::Request::builder()
				.method(method)
				.uri(uri)
				.header(http::header::CONTENT_TYPE, mime.as_ref())
				.body(body)
				.unwrap(),
		)
	}

	#[tokio::test]
	async fn test_create_user() {
		let router = router(State::new());
		let user = User {
			id: 1,
			name: "user".to_string(),
			email: "user@mail.com".to_string(),
		};

		let response = call(
			router.clone(),
			"/users",
			http::Method::POST,
			Body::from(serde_json::to_vec(&user).unwrap()),
			mime::APPLICATION_JSON,
		)
		.await
		.unwrap();

		assert_eq!(response.status(), hyper::StatusCode::CREATED);
		let res = serde_json::from_slice::<User>(
			&hyper::body::to_bytes(response.into_body()).await.unwrap(),
		)
		.unwrap();

		assert_eq!(user, res);

		let user = User {
			id: 1,
			name: "user".to_string(),
			email: "user@mail.com".to_string(),
		};

		let response = call(
			router,
			"/users",
			http::Method::POST,
			Body::from(serde_json::to_vec(&user).unwrap()),
			mime::APPLICATION_JSON,
		)
		.await
		.unwrap();

		assert_eq!(response.status(), hyper::StatusCode::CONFLICT);
	}

	#[tokio::test]
	async fn test_get_user() {
		let users = Arc::new(Mutex::new(BTreeMap::new()));
		let state = State::new_with_data(users.clone());
		let router = router(state);

		let response = call(
			router.clone(),
			&format!("/users/{}", 1),
			http::Method::GET,
			Body::empty(),
			mime::APPLICATION_JSON,
		)
		.await
		.unwrap();

		assert_eq!(response.status(), hyper::StatusCode::NOT_FOUND);

		users.lock().await.insert(
			2,
			User {
				id: 2,
				name: "alice".to_string(),
				email: "alice".to_string(),
			},
		);

		let response = call(
			router.clone(),
			&format!("/users/{}", 2),
			http::Method::GET,
			Body::empty(),
			mime::TEXT_PLAIN_UTF_8,
		)
		.await
		.unwrap();

		assert_eq!(response.status(), hyper::StatusCode::OK);
	}

	#[tokio::test]
	async fn test_delete_user() {
		let users = Arc::new(Mutex::new(BTreeMap::new()));
		let state = State::new_with_data(users.clone());
		let router = router(state);

		let response = call(
			router.clone(),
			&format!("/users/{}", 1),
			http::Method::DELETE,
			Body::empty(),
			mime::APPLICATION_JSON,
		)
		.await
		.unwrap();

		assert_eq!(response.status(), hyper::StatusCode::NOT_FOUND);

		users.lock().await.insert(
			1,
			User {
				id: 1,
				name: "alice".to_string(),
				email: "alice".to_string(),
			},
		);

		let response = call(
			router.clone(),
			&format!("/users/{}", 1),
			http::Method::DELETE,
			Body::empty(),
			mime::TEXT_PLAIN_UTF_8,
		)
		.await
		.unwrap();

		assert_eq!(response.status(), hyper::StatusCode::NO_CONTENT);

		let response = call(
			router.clone(),
			&format!("/users/{}", 1),
			http::Method::DELETE,
			Body::empty(),
			mime::TEXT_PLAIN_UTF_8,
		)
		.await
		.unwrap();

		assert_eq!(response.status(), hyper::StatusCode::NOT_FOUND);
	}

	#[tokio::test]
	async fn test_get_users() {
		let users = Arc::new(Mutex::new(BTreeMap::new()));
		let state = State::new_with_data(users.clone());
		let router = router(state);

		let response = call(
			router.clone(),
			"/users",
			http::Method::GET,
			Body::empty(),
			mime::APPLICATION_JSON,
		)
		.await
		.unwrap();

		assert_eq!(response.status(), hyper::StatusCode::OK);

		let res = serde_json::from_slice::<Vec<User>>(
			&hyper::body::to_bytes(response.into_body()).await.unwrap(),
		)
		.unwrap();

		assert_eq!(res, vec![]);

		users.lock().await.insert(
			1,
			User {
				id: 1,
				name: "111".to_string(),
				email: "111".to_string(),
			},
		);
		users.lock().await.insert(
			2,
			User {
				id: 2,
				name: "222".to_string(),
				email: "222".to_string(),
			},
		);
		users.lock().await.insert(
			3,
			User {
				id: 3,
				name: "333".to_string(),
				email: "333".to_string(),
			},
		);

		let response = call(
			router.clone(),
			"/users",
			http::Method::GET,
			Body::empty(),
			mime::APPLICATION_JSON,
		)
		.await
		.unwrap();

		assert_eq!(response.status(), hyper::StatusCode::OK);

		let res = serde_json::from_slice::<Vec<User>>(
			&hyper::body::to_bytes(response.into_body()).await.unwrap(),
		)
		.unwrap();

		assert_eq!(res.len(), 3);
	}
}
