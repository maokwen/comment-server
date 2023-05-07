#[macro_use]
extern crate rocket;
#[macro_use]
extern crate rocket_sync_db_pools;

#[cfg(test)]
mod test;

use rocket::http::Header;
use rocket::{Request, Response};
use rocket::fairing::{AdHoc, Fairing, Info, Kind};
use rocket::response::{status::Created, Debug};
use rocket::serde::{json::Json, Deserialize, Serialize};
use rocket::{Build, Rocket};
use rocket_sync_db_pools::rusqlite;

use self::rusqlite::params;

use chrono::{TimeZone, Utc, FixedOffset};

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
}

type Result<T, E = Debug<rusqlite::Error>> = std::result::Result<T, E>;

#[post("/", data = "<comment>")]
async fn post(db: Db, comment: Json<Comment>) -> Result<Created<Json<Comment>>> {
    let item = comment.clone();

    let now = Utc::now().naive_utc();
    let offset = FixedOffset::east_opt(8 * 3600).unwrap();
    let date = offset.from_utc_datetime(&now);
    let date_str = format!("{}", date.format("%Y-%m-%d %H:%M:%S"));

    db.run(move |conn| {
        conn.execute(
            "INSERT INTO comments (date, page, user, text) VALUES (?1, ?2, ?3, ?4)",
            params![date_str, item.page, item.user, item.text],
        )
    })
    .await?;

    Ok(Created::new("/").body(comment))
}

#[get("/<page>")]
async fn list(db: Db, page: String) -> Result<Json<Vec<Comment>>> {
    let comments = db.run(move |conn| {
        conn.prepare("SELECT id, date, page, user, text FROM comments WHERE page = ?1")?
        .query_map(params![page],|r| {
            Ok(Comment {
                id: Some(r.get(0)?),
                date: Some(r.get(1)?),
                page: r.get(2)?,
                user: r.get(3)?,
                text: r.get(4)?,
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
            CREATE TABLE IF NOT EXISTS comments (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                date VARCHAR NOT NULL,
                page VARCHAR NOT NULL,
                user VARCHAR NOT NULL,
                text VARCHAR NOT NULL
            );
            "#,
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
            kind: Kind::Response
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
