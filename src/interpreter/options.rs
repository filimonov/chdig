use clap::{builder::ArgPredicate, ArgAction, Args, Parser};
use std::collections::HashMap;
use std::env;
use std::num::ParseIntError;
use std::time::Duration;
use url;

#[derive(Parser, Clone)]
#[command(name = "chdig")]
#[command(author, version, about, long_about = None)]
pub struct ChDigOptions {
    #[command(flatten)]
    pub clickhouse: ClickHouseOptions,
    #[command(flatten)]
    pub view: ViewOptions,
}

#[derive(Args, Clone)]
pub struct ClickHouseOptions {
    #[arg(
        short('u'),
        long,
        value_name = "URL",
        default_value = "127.1",
        env = "CHDIG_URL"
    )]
    pub url: String,
    // Safe version for "url" (to show in UI)
    #[clap(skip)]
    pub url_safe: String,
    #[arg(short('c'), long)]
    pub cluster: Option<String>,
}

#[derive(Args, Clone)]
pub struct ViewOptions {
    #[arg(
        short('d'),
        long,
        value_parser = |arg: &str| -> Result<Duration, ParseIntError> {Ok(Duration::from_millis(arg.parse()?))},
        default_value = "3000",
    )]
    pub delay_interval: Duration,

    #[arg(short('g'), long, action = ArgAction::SetTrue, default_value_if("cluster", ArgPredicate::IsPresent, Some("true")))]
    /// Grouping distributed queries (turned on by default in --cluster mode)
    pub group_by: bool,
    #[arg(short('G'), long, action = ArgAction::SetTrue, overrides_with = "group_by")]
    no_group_by: bool,

    #[arg(long, default_value_t = false)]
    /// Do not accumulate metrics for subqueries in the initial query
    pub no_subqueries: bool,

    #[arg(short('m'), long, action = ArgAction::SetTrue, default_value_t = true)]
    /// Mouse support (turned on by default)
    pub mouse: bool,
    #[arg(short('M'), long, action = ArgAction::SetTrue, overrides_with = "mouse")]
    no_mouse: bool,
}

fn parse_url(url_str: &str) -> url::Url {
    // url::Url::scheme() does not works as we want,
    // since for "foo:bar@127.1" the scheme will be "foo",
    if url_str.contains("://") {
        return url::Url::parse(url_str).unwrap();
    }

    return url::Url::parse(&format!("tcp://{}", url_str)).unwrap();
}

fn clickhouse_url_defaults(options: &mut ChDigOptions) {
    let mut url = parse_url(&options.clickhouse.url);

    if url.username().is_empty() {
        if let Ok(env_user) = env::var("CLICKHOUSE_USER") {
            url.set_username(env_user.as_str()).unwrap();
        }
    }
    if url.password().is_none() {
        if let Ok(env_password) = env::var("CLICKHOUSE_PASSWORD") {
            url.set_password(Some(env_password.as_str())).unwrap();
        }
    }

    let mut url_safe = url.clone();

    // url_safe
    if url_safe.password().is_some() {
        url_safe.set_password(None).unwrap();
    }
    options.clickhouse.url_safe = url_safe.to_string();

    // some default settings in URL
    {
        let pairs: HashMap<_, _> = url_safe.query_pairs().into_owned().collect();
        let mut mut_pairs = url.query_pairs_mut();
        // default is: 500ms (too small)
        if !pairs.contains_key("connection_timeout") {
            mut_pairs.append_pair("connection_timeout", "5s");
        }
        // FIXME: Slow queries processing can be slow, and default timeout 180s may not be enough.
        if !pairs.contains_key("query_timeout") {
            mut_pairs.append_pair("query_timeout", "600s");
        }
    }
    options.clickhouse.url = url.to_string();
}

fn adjust_defaults(options: &mut ChDigOptions) {
    clickhouse_url_defaults(options);

    // FIXME: overrides_with works before default_value_if, hence --no-group-by never works
    if options.view.no_group_by {
        options.view.group_by = false;
    }

    // FIXME: apparently overrides_with works before default_value_t
    if options.view.no_mouse {
        options.view.mouse = false;
    }
}

// TODO:
// - config, I tried twelf but it is too buggy for now [1], let track [2] instead, I've also tried
//   viperus for the first version of this program, but it was even more buggy and does not support
//   new clap, and also it is not maintained anymore.
//
//     [1]: https://github.com/clap-rs/clap/discussions/2763
//     [2]: https://github.com/bnjjj/twelf/issues/15
//
// - clap_complete
pub fn parse() -> ChDigOptions {
    let mut options = ChDigOptions::parse();

    adjust_defaults(&mut options);

    return options;
}
