use crate::harbor::*;
use error_chain::error_chain;
use reqwest::{header::CONTENT_TYPE, Client};
use std::{cmp, collections::HashMap, sync::Arc};
use tokio::sync::Mutex;

use clap::Parser;
mod harbor;

/// 获取 Harbor 仓库的镜像版本号
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = "从 harbor 获取项目的镜像版本号")]
struct Args {
    /// harbor 中仓库的名称
    #[arg(short, long, default_value_t = String::from("smartwater"))]
    repo: String,

    /// 需要获取的项目名称, 可以多个
    #[arg(short, long, num_args=0.., default_values_t = vec![String::from("smart-water-web"), String::from("smart-water-irrigated-web")])]
    name: Vec<String>,

    /// 需要获取的最新多少个版本的镜像
    #[arg(short, long, default_value_t = 1)]
    count: usize,
}

error_chain! {
    foreign_links {
        Io(std::io::Error);
        HttpRequest(reqwest::Error);
    }
}

const HABOR_API: &'static str = "http://10.12.135.233/api";

fn get_full_url(repo: &str, name: &str) -> String {
    format!(
        "{url}/repositories/{repo}/{name}/tags?detail={detail}",
        url = HABOR_API,
        repo = repo,
        name = name,
        detail = true
    )
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    // println!("args: {:#?}", args);

    let map: Arc<Mutex<HashMap<String, Vec<String>>>> = Arc::new(Mutex::new(HashMap::new()));

    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()?;

    let mut tasks = Vec::with_capacity(args.name.len());
    for name in args.name {
        tasks.push(tokio::spawn(make_request(
            args.repo.clone(),
            name.clone(),
            args.count,
            client.clone(),
            map.clone(),
        )));
    }

    for task in tasks {
        match task.await {
            Ok(_) => {}
            Err(_) => {
                panic!("tokio task error");
            }
        };
    }

    // println!("map is {:#?}", map);
    // print map by key
    let lock = map.lock().await;
    for (key, value) in lock.iter() {
        println!("\n{}:", key);
        for s in value {
            println!("  {}", s);
        }
    }
    Ok(())
}

async fn make_request(
    repo: String,
    name: String,
    count: usize,
    client: Client,
    map: Arc<Mutex<HashMap<String, Vec<String>>>>,
) -> Result<()> {
    let full_url = get_full_url(&repo, &name);

    let res = client
        .get(full_url)
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

                let s = format!("{:6}: {}/{} {}", label, repo, name, detail.name);

                label_list.push(s);
            }
            let mut lock = map.lock().await;
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
