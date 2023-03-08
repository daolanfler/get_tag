use crate::harbor::*;
use error_chain::error_chain;
use reqwest::header::CONTENT_TYPE;

use clap::Parser;
mod harbor;

/// 获取 Harbor 仓库的镜像版本号
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = "从 harbor 获取项目的镜像版本号")]
struct Args {
    /// harbor 中仓库的名称
    #[arg(short, long, default_value_t = String::from("smartwater"))]
    repo: String,

    /// 需要获取的项目名称
    #[arg(short, long, default_value_t = String::from("smart-water-web"))]
    name: String,

    /// 需要获取的最新多少个版本的镜像
    #[arg(short, long, default_value_t = 1)]
    count: u8,
}

error_chain! {
    foreign_links {
        Io(std::io::Error);
        HttpRequest(reqwest::Error);
    }
}

const HABOR_API: &'static str = "http://10.12.135.233/api";

fn get_full_url(repo: &str, name: &str, detail: bool) -> String {
    format!(
        "{url}/repositories/{repo}/{name}/tags?detail={detail}",
        url = HABOR_API,
        repo = repo,
        name = name,
        detail = detail
    )
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // detail 为 false 则返回结果不同
    let full_url = get_full_url(&args.repo, &args.name, true);
    println!("Full url: {}", full_url);
    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()?;

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
            let first = detail_list.first();
            match first {
                Some(detail) => {
                    println!("最新版本 tag: {}/{} {}", args.repo, args.name, detail.name)
                }
                None => {
                    println!("No images found")
                }
            }
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
