use rocket::http::{ContentType, Status};
use rocket::request::{self, Request, FromRequest};
use rocket::response;
use rocket::response::{Responder, Response};
use rocket_contrib::json::{Json, JsonValue};
use rocket::Outcome;
use google_signin;
use std::env;

use crate::lib::subscriber::Subscriber;
use crate::lib::subscriber::Friend;
use crate::lib::subscriber::Profile;
use crate::lib::class::Class;
use crate::lib::class::Item;
use crate::DbConn;

#[derive(Debug)]
pub struct ApiResponse {
    json: JsonValue,
    status: Status
}

#[derive(Debug)]
pub struct Bearer (String); 

/// Returns true if `key` is a valid API key string.
fn is_valid(_key: &str) -> bool {
	true
}

impl<'r> Responder<'r> for ApiResponse {
    fn respond_to(self, req: &Request) -> response::Result<'r> {
        Response::build_from(self.json.respond_to(&req).unwrap())
            .status(self.status)
            .header(ContentType::JSON)
            .ok()
    }
}

impl<'a, 'r> FromRequest<'a, 'r> for Bearer {
    type Error = JsonValue;

    fn from_request(request: &'a Request<'r>) -> request::Outcome<Self, Self::Error> {
        let keys: Vec<_> = request.headers().get("Authorization").collect();
        match keys.len() {
            0 => Outcome::Failure((Status::BadRequest, json!({ "error": "Missing Auth" }))),
            1 if is_valid(keys[0]) => Outcome::Success(Bearer(keys[0][7..].to_string())),
            1 => Outcome::Failure((Status::BadRequest, json!({ "error": "Invalid Auth" }))),
            _ => Outcome::Failure((Status::BadRequest, json!({ "error": "Unknown Auth" }))),
        }
    }
}

#[post("/subscriber/register", format = "application/json")]
pub fn subscriber_register(conn: DbConn, key: Result<Bearer, JsonValue>) -> ApiResponse {
	let mut client = google_signin::Client::new();
	client.audiences.push(env::var("GGL_KEY").unwrap().to_string()); // required
	match key {
        Ok(bearer) => {
			let id_info = client.verify(&bearer.0).expect("Expected token to be valid");
			println!("Success! Signed-in as {}", id_info.sub);
			let mut subscriber: Subscriber = Subscriber{id:None,
														social_id:Some(id_info.sub),
														proto:Some("google".to_string())
													};	
			match subscriber.create(conn) {
				Ok(()) => return ApiResponse {
							json: json!(subscriber),
							status: Status::Ok,
						},
				Err(e) => return ApiResponse {
							json: json!({"message": String::from("Subscriber couldn't be created: ")+&e.details}),
							status: Status::UnprocessableEntity,
						}
			}
		},
		Err(json_error) => return ApiResponse {
			json: json_error,
			status: Status::BadRequest
		}
	};
}

#[get("/class/<id>/<subscriber_id>", format = "application/json")]
pub fn get_class(conn: DbConn, id: String, _key: Result<Bearer, JsonValue>, subscriber_id:String) -> ApiResponse {
	let subscriber: Subscriber = Subscriber{id:Some(subscriber_id),
												social_id:None,
												proto:None};
	let mut class: Class = Class(id);
	match class.get_class(conn, &subscriber.id.unwrap()) {
		Ok(items) => return ApiResponse {
					json: json!(items),
					status: Status::Ok,
				},
		Err(e) => return ApiResponse {
					json: json!({"message": String::from("Items failed: ")+&e.details}),
					status: Status::NotFound,
				}
	}
}

#[post("/class/<id>", format = "application/json", data="<items>")]
pub fn post_class(conn: DbConn, id: String, key: Result<Bearer, JsonValue>, items: Json<Vec<Item>>) -> ApiResponse {
	match key {
		Ok(bearer) => {
			let mut client = google_signin::Client::new();
			client.audiences.push(env::var("GGL_KEY").unwrap());
			let id_info = match client.verify(&bearer.0) {
				Ok(res) => res,
				Err(_) => return ApiResponse {
										json: json!({"message": "Unauthorized user"}),
										status: Status::Unauthorized,
									}
			};
			let mut subscriber: Subscriber = Subscriber{
											id:None,
											social_id:Some(id_info.sub),
											proto:Some("google".to_string())
											};
			match subscriber.set_id(&conn) {
				Ok(()) => {
						let mut class: Class = Class(id);
						match class.set_class(conn, &subscriber.id.unwrap(), items.into_inner()) {
							Ok(_items) => return ApiResponse {
										json: json!({"message": "Success"}),
										status: Status::Ok,
									},
							Err(e) => return ApiResponse {
										json: json!({"message": String::from("Items failed: ")+&e.details}),
										status: Status::NotFound,
									}
						}
					},
				Err(_e) => return ApiResponse {
					json: json!({"message":"Subscriber not found"}),
					status: Status::NotFound,
				}
			}
		},
		Err(json_error) => return ApiResponse {
			json: json_error,
			status: Status::BadRequest
		}
	};
}

#[get("/friends", format = "application/json")]
pub fn get_friends(conn: DbConn, key: Result<Bearer, JsonValue>) -> ApiResponse {
	match key {
		Ok(bearer) => {
			let mut client = google_signin::Client::new();
			client.audiences.push(env::var("GGL_KEY").unwrap());
			let id_info = match client.verify(&bearer.0) {
				Ok(res) => res,
				Err(_) => return ApiResponse {
										json: json!({"message": "Unauthorized user"}),
										status: Status::Unauthorized,
									}
			};
			let mut subscriber: Subscriber = Subscriber{
											id:None,
											social_id:Some(id_info.sub),
											proto:Some("google".to_string())
											};
			match subscriber.set_id(&conn) {
				Ok(()) => {
					match subscriber.get_friends(&conn) {
						Ok(friends) => return ApiResponse {
								json: json!(friends),
								status: Status::Ok
						},
						Err(e) => return ApiResponse {
									json: json!({"message": String::from("Friends failed: ")+&e.details}),
									status: Status::NotFound,
								}
					}
				},
				Err(_e) => return ApiResponse {
					json: json!({"message":"Subscriber not found"}),
					status: Status::NotFound,
				}
			}
		},
		Err(json_error) => return ApiResponse {
			json: json_error,
			status: Status::BadRequest
		}
	};
}

#[post("/friend", format = "application/json", data="<friend>")]
pub fn post_friend(conn: DbConn, key: Result<Bearer, JsonValue>, friend: Json<Friend>) -> ApiResponse {
	match key {
		Ok(bearer) => {
			let mut client = google_signin::Client::new();
			client.audiences.push(env::var("GGL_KEY").unwrap());
			let id_info = match client.verify(&bearer.0) {
				Ok(res) => res,
				Err(_) => return ApiResponse {
										json: json!({"message": "Unauthorized user"}),
										status: Status::Unauthorized,
									}
			};
			let mut subscriber: Subscriber = Subscriber{
											id:None,
											social_id:Some(id_info.sub),
											proto:Some("google".to_string())
											};
			match subscriber.set_id(&conn) {
				Ok(()) => {
					match subscriber.set_friend(&conn, friend.into_inner()) {
						Ok(_items) => return ApiResponse {
									json: json!({"message": "Success"}),
									status: Status::Ok,
								},
						Err(e) => return ApiResponse {
									json: json!({"message": String::from("Friend failed: ")+&e.details}),
									status: Status::NotFound,
								}
					}
				},
				Err(_e) => return ApiResponse {
					json: json!({"message":"Subscriber not found"}),
					status: Status::NotFound,
				}
			}
		},
		Err(json_error) => return ApiResponse {
			json: json_error,
			status: Status::BadRequest
		}
	};
}

#[put("/friend", format = "application/json", data="<friend>")]
pub fn update_friend(conn: DbConn, key: Result<Bearer, JsonValue>, friend: Json<Friend>) -> ApiResponse {
	match key {
		Ok(bearer) => {
			let mut client = google_signin::Client::new();
			client.audiences.push(env::var("GGL_KEY").unwrap());
			let id_info = match client.verify(&bearer.0) {
				Ok(res) => res,
				Err(_) => return ApiResponse {
										json: json!({"message": "Unauthorized user"}),
										status: Status::Unauthorized,
									}
			};
			let mut subscriber: Subscriber = Subscriber{
											id:None,
											social_id:Some(id_info.sub),
											proto:Some("google".to_string())
											};
			match subscriber.set_id(&conn) {
				Ok(()) => {
					match subscriber.update_friend(&conn, friend.into_inner()) {
						Ok(_items) => return ApiResponse {
									json: json!({"message": "Success"}),
									status: Status::Ok,
								},
						Err(e) => return ApiResponse {
									json: json!({"message": String::from("Friend failed: ")+&e.details}),
									status: Status::NotFound,
								}
					}
				},
				Err(_e) => return ApiResponse {
					json: json!({"message":"Subscriber not found"}),
					status: Status::NotFound,
				}
			}
		},
		Err(json_error) => return ApiResponse {
			json: json_error,
			status: Status::BadRequest
		}
	};
}

#[delete("/friend/<id>", format = "application/json")]
pub fn delete_friend(conn: DbConn, key: Result<Bearer, JsonValue>, id: String ) -> ApiResponse {
	match key {
		Ok(bearer) => {
			let mut client = google_signin::Client::new();
			client.audiences.push(env::var("GGL_KEY").unwrap());
			let id_info = match client.verify(&bearer.0) {
				Ok(res) => res,
				Err(_) => return ApiResponse {
										json: json!({"message": "Unauthorized user"}),
										status: Status::Unauthorized,
									}
			};
			let mut subscriber: Subscriber = Subscriber{
											id:None,
											social_id:Some(id_info.sub),
											proto:Some("google".to_string())
											};
			match subscriber.set_id(&conn) {
				Ok(()) => {
					match subscriber.delete_friend(&conn, id) {
						Ok(_items) => return ApiResponse {
									json: json!({"message": "Success"}),
									status: Status::Ok,
								},
						Err(e) => return ApiResponse {
									json: json!({"message": String::from("Friend failed: ")+&e.details}),
									status: Status::NotFound,
								}
					}
				},
				Err(_e) => return ApiResponse {
					json: json!({"message":"Subscriber not found"}),
					status: Status::NotFound,
				}
			}
		},
		Err(json_error) => return ApiResponse {
			json: json_error,
			status: Status::BadRequest
		}
	};
}

#[get("/check/friend/<id>", format = "application/json")]
pub fn check_friend(conn: DbConn, key: Result<Bearer, JsonValue>, id: String) -> ApiResponse {
	match key {
		Ok(bearer) => {
			let mut client = google_signin::Client::new();
			client.audiences.push(env::var("GGL_KEY").unwrap());
			let id_info = match client.verify(&bearer.0) {
				Ok(res) => res,
				Err(_) => return ApiResponse {
										json: json!({"message": "Unauthorized user"}),
										status: Status::Unauthorized,
									}
			};
			let mut subscriber: Subscriber = Subscriber{
											id:None,
											social_id:Some(id_info.sub),
											proto:Some("google".to_string())
											};
			match subscriber.set_id(&conn) {
				Ok(()) => {
					match subscriber.check_friend(&conn, &id) {
						Ok(name) => return ApiResponse {
									json: json!({"name": name}),
									status: Status::Ok,
								},
						Err(e) => return ApiResponse {
									json: json!({"message": String::from("Friend failed: ")+&e.details}),
									status: Status::NotFound,
								}
					}
				},
				Err(_e) => return ApiResponse {
					json: json!({"message":"Subscriber not found"}),
					status: Status::NotFound,
				}
			}
		},
		Err(json_error) => return ApiResponse {
			json: json_error,
			status: Status::BadRequest
		}
	};
}

#[post("/profile", format = "application/json", data="<profile>")]
pub fn post_profile(conn: DbConn, key: Result<Bearer, JsonValue>, profile: Json<Profile>) -> ApiResponse {
	match key {
		Ok(bearer) => {
			let mut client = google_signin::Client::new();
			client.audiences.push(env::var("GGL_KEY").unwrap());
			let id_info = match client.verify(&bearer.0) {
				Ok(res) => res,
				Err(_) => return ApiResponse {
										json: json!({"message": "Unauthorized user"}),
										status: Status::Unauthorized,
									}
			};
			let mut subscriber: Subscriber = Subscriber{
											id:None,
											social_id:Some(id_info.sub),
											proto:Some("google".to_string())
											};
			match subscriber.set_id(&conn) {
				Ok(()) => {
					match subscriber.set_profile(&conn, profile.into_inner()) {
						Ok(_items) => return ApiResponse {
									json: json!({"message": "Success"}),
									status: Status::Ok,
								},
						Err(e) => return ApiResponse {
									json: json!({"message": String::from("Profile failed: ")+&e.details}),
									status: Status::NotFound,
								}
					}
				},
				Err(_e) => return ApiResponse {
					json: json!({"message":"Subscriber not found"}),
					status: Status::NotFound,
				}
			}
		},
		Err(json_error) => return ApiResponse {
			json: json_error,
			status: Status::BadRequest
		}
	};
}

#[get("/profile", format = "application/json")]
pub fn get_profile(conn: DbConn, key: Result<Bearer, JsonValue>) -> ApiResponse {
	match key {
		Ok(bearer) => {
			let mut client = google_signin::Client::new();
			client.audiences.push(env::var("GGL_KEY").unwrap());
			let id_info = match client.verify(&bearer.0) {
				Ok(res) => res,
				Err(_) => return ApiResponse {
										json: json!({"message": "Unauthorized user"}),
										status: Status::Unauthorized,
									}
			};
			let mut subscriber: Subscriber = Subscriber{
											id:None,
											social_id:Some(id_info.sub),
											proto:Some("google".to_string())
											};
			match subscriber.set_id(&conn) {
				Ok(()) => {
					match subscriber.get_profile(&conn) {
						Ok(profile) => return ApiResponse {
								json: json!(profile),
								status: Status::Ok
						},
						Err(e) => return ApiResponse {
									json: json!({"message": String::from("Profile failed: ")+&e.details}),
									status: Status::NotFound,
								}
					}
				},
				Err(_e) => return ApiResponse {
					json: json!({"message":"Subscriber not found"}),
					status: Status::NotFound,
				}
			}
		},
		Err(json_error) => return ApiResponse {
			json: json_error,
			status: Status::BadRequest
		}
	};
}

#[get("/profile/<id>", format = "application/json")]
pub fn get_other_profile(conn: DbConn, id: String, key: Result<Bearer, JsonValue>) -> ApiResponse {
	match key {
		Ok(bearer) => {
			let mut client = google_signin::Client::new();
			client.audiences.push(env::var("GGL_KEY").unwrap());
			let id_info = match client.verify(&bearer.0) {
				Ok(res) => res,
				Err(_) => return ApiResponse {
										json: json!({"message": "Unauthorized user"}),
										status: Status::Unauthorized,
									}
			};
			let mut subscriber: Subscriber = Subscriber{
											id:None,
											social_id:Some(id_info.sub),
											proto:Some("google".to_string())
											};
			match subscriber.set_id(&conn) {
				Ok(()) => {
					match subscriber.get_other_profile(&conn, &id) {
						Ok(profile) => return ApiResponse {
								json: json!(profile),
								status: Status::Ok
						},
						Err(e) => return ApiResponse {
									json: json!({"message": String::from("Profile failed: ")+&e.details}),
									status: Status::NotFound,
								}
					}
				},
				Err(_e) => return ApiResponse {
					json: json!({"message":"Subscriber not found"}),
					status: Status::NotFound,
				}
			}
		},
		Err(json_error) => return ApiResponse {
			json: json_error,
			status: Status::BadRequest
		}
	};
}
