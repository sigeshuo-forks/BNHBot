use chrono::{DateTime, Utc, Local, Timelike};

pub fn format_local_time(datetime: DateTime<Utc>) -> String {
    let local_time = datetime.with_timezone(&Local);
    local_time.format("%Y-%m-%d %H:%M:%S").to_string()
}

pub fn get_beijing_time() -> DateTime<Utc> {
    // 北京时间 UTC+8
    Utc::now() + chrono::Duration::hours(8)
}

pub fn is_market_open() -> bool {
    // 简单的市场开放时间判断（UTC时间）
    let now = Utc::now();
    let hour = now.hour();
    
    // 假设市场在UTC 0:00-16:00开放（对应北京时间8:00-24:00）
    hour < 16
}
