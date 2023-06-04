use super::rocket;
use rocket::local::blocking::Client;
use rocket::serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(crate = "rocket::serde")]
struct Comment {
    page: String,
    user: String,
    text: String,
}

#[test]
fn test() {
    let client = Client::tracked(rocket()).unwrap();

    let count = client
        .get("/aaa")
        .dispatch()
        .into_json::<Vec<Comment>>()
        .unwrap().len();

    // test post
    for i in 1_usize..=20 {
        let page = "aaa".to_string();
        let user = "user1".to_string();
        let text = format!("msg msg {}", i);
        let msg = Comment { page, user, text };

        let response = client
            .post("/")
            .json(&msg)
            .dispatch()
            .into_json::<Comment>();
        assert_eq!(response.unwrap(), msg);

        let list = client
            .get("/aaa")
            .dispatch()
            .into_json::<Vec<Comment>>()
            .unwrap();
        assert_eq!(list.len(), count + i);
    }
}
