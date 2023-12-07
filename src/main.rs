mod harbor;

use crate::harbor::*;
use chrono::Local;
use error_chain::error_chain;
use reqwest::{header::CONTENT_TYPE, Client};
use std::{
    cmp,
    collections::HashMap,
    sync::{Arc, Mutex},
};
use clap::{Parser, Subcommand};

/// 获取镜像版本号
#[derive(Parser, Debug)]
#[command(
    author = "daolanfler",
    version,
    long_about = "从 harbor 获取项目的镜像版本号",
    // help_template = "{name} ({version}) by {author} {about} \n\n{usage}"
)]
struct GetTagArgs {
    /// harbor 中仓库的名称
    #[arg(short, long, default_value_t = String::from("smartwater"))]
    repo: String,

    /// 需要获取的项目名称, 可以多个
    #[arg(short, long, num_args=0.., default_values_t = vec![String::from("smart-water-web"), String::from("smart-water-irrigated-web")], value_name="PROJECT_NAME")]
    names: Vec<String>,

    /// 需要获取的最新多少个版本的镜像
    #[arg(short, long, default_value_t = 1)]
    count: usize,

    /// 是否打印推送时间
    #[arg(short, long, default_value_t = false)]
    time: bool,

    /// 打印 harbor API 地址
    #[command(subcommand)]
    command: Option<Commands>,
}

// for subcommand examples
// https://docs.rs/clap/latest/clap/_derive/_tutorial/chapter_0/index.html

#[derive(Subcommand, Debug)]
enum Commands{
    /// 打印 harbor API 地址
    Harbor
}


error_chain! {
    foreign_links {
        Io(std::io::Error);
        HttpRequest(reqwest::Error);
        JoinError(tokio::task::JoinError);
    }
}

const HABOR_API_URL: &str = "http://10.12.135.233/api";

fn get_full_url(repo: &str, name: &str) -> String {
    format!(
        "{url}/repositories/{repo}/{name}/tags?detail={detail}",
        url = HABOR_API_URL,
        repo = repo,
        name = name,
        detail = true
    )
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Arc::new(GetTagArgs::parse());

    #[cfg(debug_assertions)] println!("args: {:#?}", args);

    match args.command {
        Some(Commands::Harbor) => {
            println!("The current harbor API is: {}", HABOR_API_URL);
            return Ok(());
        }
        None => {}
    }

    let map: Arc<Mutex<HashMap<String, Vec<String>>>> = Arc::new(Mutex::new(HashMap::new()));

    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()?;

    let mut tasks = Vec::with_capacity(args.names.len());
    for name in args.names.iter() {
        // tokio spawn 的 future 会立即执行
        let task = tokio::spawn(make_request(
            args.clone(),
            name.clone(),
            client.clone(),
            map.clone(),
        ));
        tasks.push(task);
    }

    for task in tasks {
        task.await.map_err(|e| dbg!(e))?.map_err(|e| {
            println!("shit happend");
            return e;
        })?;
    }

    // println!("map is {:#?}", map);
    // print map by key
    let lock = map.lock().unwrap();
    for (key, tag_list) in lock.iter() {
        println!("\n{}:", key);
        for s in tag_list {
            println!("  {}", s);
        }
    }
    Ok(())
}

async fn make_request(
    args: Arc<GetTagArgs>,
    name: String,
    client: Client,
    map: Arc<Mutex<HashMap<String, Vec<String>>>>,
) -> Result<()> {
    let count = args.count;
    let repo = &args.repo;
    let print_time = args.time;

    let full_url = get_full_url(repo, &name);

    let res = client
        .get(&full_url)
        .header(CONTENT_TYPE, "application/json")
        .send()
        .await?;

    match res.status() {
        reqwest::StatusCode::OK => {
            let mut detail_list = res.json::<Vec<ProjectDetail>>().await?;

            detail_list.sort_by_key(|detail| detail.push_time);

            detail_list.reverse();
            let end_index = cmp::min(count, detail_list.len());
            let required_list = detail_list[0..end_index].to_vec();

            let mut label_list = vec![];
            for (index, detail) in required_list.iter().enumerate() {
                let label = if index == 0 {
                    "newest".to_string()
                } else {
                    format!("No.{} ", index + 1)
                };

                let mut s = format!("{:6}: {}/{} {}", label, repo, &name, detail.name,);
                if print_time {
                    s = format!(
                        "{}  推送时间：{}",
                        s,
                        detail
                            .push_time
                            .with_timezone(&Local)
                            .format("%Y-%m-%d %H:%M:%S")
                    );
                }

                label_list.push(s);
            }
            let mut lock = map.lock().unwrap();
            lock.insert(name, label_list);
        }
        reqwest::StatusCode::UNAUTHORIZED => {
            println!("UNAUTHORIZED");
        }
        _ => {
            panic!("出现了未知错误")
        }
    }
    Ok(())
}
