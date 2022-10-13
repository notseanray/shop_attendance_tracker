use chrono::{Datelike, Local, Timelike};
use eframe::egui;
use rusqlite::{Connection, Result, Row};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Deserialize)]
struct Config {
    admin_pass: String,
    database_path: String,
}

fn main() {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "CR Shop Attendance",
        options,
        Box::new(|_cc| Box::new(MyApp::new())),
    );
}

struct MyApp {
    name: String,
    admin: bool,
    export_len: usize,
    conf: Config,
    database: Connection,
}

impl MyApp {
    pub fn new() -> Self {
        let config = match fs::read_to_string("./config.json") {
            Ok(v) => v,
            Err(_) => {
                let _ = File::create("config.json").expect("no fs access");
                println!("please fill out config.json with json config values for `admin_pass` and `database_path`");
                std::process::exit(1);
            }
        };
        let config = serde_json::from_str(&config);
        if let Err(e) = config {
            println!("{:?}", e);
            println!("failed to read config");
            std::process::exit(1);
        };
        let conf: Config = config.unwrap();
        Self {
            name: "Joe".to_owned(),
            admin: false,
            database: Connection::open(&conf.database_path).expect("failed to connect to DB"),
            export_len: 1,
            conf,
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("CR Sign in");
            ui.label("Enter with the format FirstName LastName GradYear:");
            let dt = Local::now();
            ui.label(format!("Example: John Doe {}", dt.year()));
            ui.text_edit_singleline(&mut self.name);
            if ui.button("Submit").clicked() {
                // db
                if let Ok(v) = NameTag::from_str(&self.name) {
                    let t: NameTagDB = v.into();
                    let _ = self.database.execute(
                        "insert into attendance_data values (?1, ?2, ?3, ?4, ?5, ?6)",
                        (
                            t.ID,
                            t.first_name,
                            t.last_name,
                            t.grad_year,
                            t.badge,
                            t.creation_date,
                        ),
                    );
                }
                self.name.clear();
            }
            ui.label({
                self.admin = self.name == self.conf.admin_pass;
                if self.name.ends_with('%') {
                    if let Ok(v) = NameTag::from_str(&self.name) {
                        let t: NameTagDB = v.into();
                        let _ = self.database.execute(
                            "insert into attendance_data values (?1, ?2, ?3, ?4, ?5, ?6);",
                            (
                                t.ID,
                                t.first_name,
                                t.last_name,
                                t.grad_year,
                                t.badge,
                                t.creation_date,
                            ),
                        );
                    }
                    self.name.clear();
                }
                let text: Vec<&str> = self.name.split_whitespace().collect();
                let mut name = self.name.clone();
                if !name.is_empty() {
                    name = format!("Welcome {name}");
                }
                let grad_year = match text.len() {
                    3 => {
                        if let Ok(v) = text[2].parse::<u16>() {
                            name = format!("Welcome {} {}", text[0], text[1]);
                            format!(", Graduation year: {v}")
                        } else {
                            String::from("")
                        }
                    }
                    _ => String::from(""),
                };
                format!("{name}{grad_year}")
            });
            if self.admin {
                // ui.add(egui::Slider::new(&mut self.export_len, 1..=180).text("Days to export"));
                if ui.button("Export to JSON").clicked() {
                    let mut data = self
                        .database
                        .prepare("select * from attendance_data;")
                        .expect("db connection failure");
                    let records = data
                        .query_map([], |row| Ok(NameTagDB::from_row(row)))
                        .unwrap();
                    let _ = fs::create_dir_all("dumps");
                    let dt = Local::now();
                    let mut f = OpenOptions::new().append(true).create(true).open(format!(
                        "dumps/{}{}{}{}.json",
                        dt.year(),
                        dt.month(),
                        dt.day(),
                        dt.minute()
                    ));
                    for r in records {
                        if let Ok(v) = &mut f {
                            let _ = v.write(
                                serde_json::to_string(&r.unwrap().unwrap())
                                    .unwrap()
                                    .as_bytes(),
                            );
                        }
                    }
                }
            }
        });
    }
}

struct NameTag {
    first_name: String,
    last_name: String,
    grad_year: u16,
    badge: bool,
}

enum NameTagErr {
    DeserializeError,
    SeralizeError,
    DBError,
}

impl NameTagDB {
    pub fn from_row(row: &Row) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            ID: row.get(0)?,
            first_name: row.get(1)?,
            last_name: row.get(2)?,
            grad_year: row.get(3)?,
            badge: row.get(4)?,
            creation_date: row.get(5)?,
        })
    }
}

impl FromStr for NameTag {
    type Err = NameTagErr;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.to_lowercase();
        if s.ends_with('%') {
            let tag: Vec<&str> = s.split('$').collect();
            println!("{:?}", tag);
            let grad_year = tag[2][..tag.len() - 1].parse::<u16>().unwrap_or(0);
            if tag.len() == 3 && grad_year > 0 {
                return Ok(Self {
                    first_name: tag[0].to_string(),
                    last_name: tag[1].to_string(),
                    grad_year,
                    badge: true,
                });
            }
        }
        let tag: Vec<&str> = s.split_whitespace().collect();
        if tag.len() != 3 {
            return Err(NameTagErr::DeserializeError);
        }
        if let Ok(v) = tag[2].parse::<u16>() {
            Ok(Self {
                first_name: tag[0].to_string(),
                last_name: tag[1].to_string(),
                grad_year: v,
                badge: false,
            })
        } else {
            Err(NameTagErr::DeserializeError)
        }
    }
}

enum DBError {
    InsertionError,
    ExtractionError,
}

impl std::fmt::Debug for DBError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        type D = DBError;
        match self {
            D::InsertionError => write!(f, "Failed to insert element into DB"),
            D::ExtractionError => write!(f, "Failed to extract elements from DB"),
        }
    }
}

#[allow(non_snake_case)]
#[derive(Deserialize, Serialize)]
struct NameTagDB {
    ID: String,
    first_name: String,
    last_name: String,
    grad_year: i64,
    badge: bool,
    creation_date: String,
}

impl From<NameTag> for NameTagDB {
    fn from(t: NameTag) -> Self {
        let dt = Local::now();
        Self {
            ID: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock went backwards")
                .as_millis()
                .to_string(),
            first_name: t.first_name,
            last_name: t.last_name,
            grad_year: t.grad_year as i64,
            badge: t.badge,
            creation_date: format!("{}/{}/{}", dt.day(), dt.month(), dt.year()),
        }
    }
}
