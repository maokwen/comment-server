#[macro_use]
extern crate rocket;
#[macro_use]
extern crate rocket_sync_db_pools;

#[cfg(test)]
mod test;

use rocket::fairing::{AdHoc, Fairing, Info, Kind};
use rocket::http::Header;
use rocket::log::private::{warn, info};
use rocket::response::{status::Created, Debug};
use rocket::serde::{json::Json, Deserialize, Serialize};
use rocket::{Build, Rocket};
use rocket::{Request, Response};
use rocket_sync_db_pools::rusqlite;
use std::result::Result;
use self::rusqlite::params;
use chrono::{FixedOffset, TimeZone, Utc};

#[database("comment_db")]

struct Db(rusqlite::Connection);

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(crate = "rocket::serde")]
struct Comment {
    #[serde(skip_deserializing, skip_serializing_if = "Option::is_none")]
    id: Option<i64>,
    date: Option<String>,
    page: String,
    user: String,
    text: String,
    email: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct Config {
    push: PushConfig,
}

#[derive(Deserialize, Debug)]
pub struct PushConfig {
    pub url: String,
    pub device_key: String,
}

fn read_config(filename: String) -> Result<Config, Box<dyn std::error::Error>> {
    let content = std::fs::read_to_string(filename)?;
    Ok(toml::from_str(&content)?)
}

#[macro_use]
extern crate lazy_static;
lazy_static! {
    pub static ref CONFIG: Config = read_config("Config.toml".to_owned()).unwrap();
}

#[derive(Serialize, Deserialize)]
pub struct PushData {
    pub title: String,
    pub body: String,
    pub device_key: String,
}

async fn notify(comment: &Json<Comment>) -> Result<(), Box<dyn std::error::Error>> {
    // push notification
    let title = format!("{} on {}", &comment.user, &comment.page);
    let body = comment.text.clone();
    let device_key = CONFIG.push.device_key.clone();
    let data = PushData {
        title,
        body,
        device_key,
    };

    let json_str = serde_json::to_string(&data)?;

    info!("push notification: {}", &json_str);

    // make a post request to push notification server
    let resp: reqwest::Response = reqwest::Client::new()
        .post(CONFIG.push.url.as_str())
        .header("Content-Type", "application/json; charset=utf-8")
        .body(json_str)
        .send()
        .await?;

    if resp.error_for_status_ref().is_err() {
        let status = resp.status();
        let text = resp.text().await?;
        warn!("{}: {}", status.as_str(), text.as_str());
    }

    Ok(())
}

#[post("/", data = "<comment>")]
async fn post(db: Db, comment: Json<Comment>) -> Result<Created<Json<Comment>>, Debug<rusqlite::Error>> {
    let item = comment.clone();

    let now = Utc::now().naive_utc();
    let offset = FixedOffset::east_opt(8 * 3600).unwrap();
    let date = offset.from_utc_datetime(&now);
    let date_str = format!("{}", date.format("%Y-%m-%d %H:%M:%S"));

    db.run(move |conn| {
        conn.execute(
            "INSERT INTO comments (date, page, user, text, email) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![date_str, item.page, item.user, item.text, item.email],
        )
    })
    .await?;

    info!("inserted: {:?}", comment);
    // try post notification
    if notify(&comment).await.is_err() {
        warn!("push notification failed");
    }

    Ok(Created::new("/").body(comment))
}

#[get("/<page>")]
async fn list(db: Db, page: String) -> Result<Json<Vec<Comment>>, Debug<rusqlite::Error>> {
    let comments = db
        .run(move |conn| {
            conn.prepare("SELECT id, date, page, user, text FROM comments WHERE page = ?1")?
                .query_map(params![page], |r| {
                    Ok(Comment {
                        id: Some(r.get(0)?),
                        date: Some(r.get(1)?),
                        page: r.get(2)?,
                        user: r.get(3)?,
                        text: r.get(4)?,
                        email: None,
                    })
                })?
                .collect::<Result<Vec<Comment>, _>>()
        })
        .await?;

    Ok(Json(comments))
}

async fn init_db(rocket: Rocket<Build>) -> Rocket<Build> {
    Db::get_one(&rocket)
        .await
        .expect("database mounted")
        .run(|conn| {
            conn.execute(
                r#"
            CREATE TABLE IF NOT EXISTS "comments" (
                "id"	INTEGER,
                "date"	TEXT NOT NULL,
                "page"	TEXT NOT NULL,
                "user"	TEXT NOT NULL,
                "text"	TEXT NOT NULL,
                "email"	TEXT,
                PRIMARY KEY("id" AUTOINCREMENT)
            );"#,
                params![],
            )
        })
        .await
        .expect("can init rusqlite DB");

    rocket
}

pub struct CORS;

#[rocket::async_trait]
impl Fairing for CORS {
    fn info(&self) -> Info {
        Info {
            name: "Add CORS headers to responses",
            kind: Kind::Response,
        }
    }

    async fn on_response<'r>(&self, _request: &'r Request<'_>, response: &mut Response<'r>) {
        response.set_header(Header::new("Access-Control-Allow-Origin", "*"));
        response.set_header(Header::new("Access-Control-Allow-Methods", "POST, GET, PATCH, OPTIONS"));
        response.set_header(Header::new("Access-Control-Allow-Headers", "*"));
        response.set_header(Header::new("Access-Control-Allow-Credentials", "true"));
    }
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .attach(Db::fairing())
        .attach(AdHoc::on_ignite("Rusqlite Init", init_db))
        .attach(CORS)
        .mount("/", routes![post, list])
}
