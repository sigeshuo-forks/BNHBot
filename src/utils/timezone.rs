use chrono::{DateTime, Utc, FixedOffset, TimeZone};

/// 中国时区常量 (UTC+8)
pub const CHINA_TZ: i32 = 8 * 3600;

/// 时区转换工具
pub struct TimezoneUtil;

impl TimezoneUtil {
    /// 获取中国时区
    pub fn china_timezone() -> FixedOffset {
        FixedOffset::east_opt(CHINA_TZ).unwrap()
    }
    
    /// 将UTC时间转换为中国时间
    pub fn utc_to_china(utc_time: DateTime<Utc>) -> DateTime<FixedOffset> {
        utc_time.with_timezone(&Self::china_timezone())
    }
    
    /// 获取当前中国时间
    pub fn now_china() -> DateTime<FixedOffset> {
        Self::utc_to_china(Utc::now())
    }
    
    /// 将中国时间转换为UTC时间
    pub fn china_to_utc(china_time: DateTime<FixedOffset>) -> DateTime<Utc> {
        china_time.with_timezone(&Utc)
    }
    
    /// 格式化中国时间为字符串
    pub fn format_china_time(time: DateTime<FixedOffset>) -> String {
        time.format("%Y-%m-%d %H:%M:%S CST").to_string()
    }
    
    /// 格式化当前中国时间
    pub fn format_now_china() -> String {
        Self::format_china_time(Self::now_china())
    }
    
    /// 创建中国时间的特定时间点
    pub fn china_time(year: i32, month: u32, day: u32, hour: u32, min: u32, sec: u32) -> Option<DateTime<FixedOffset>> {
        let china_tz = Self::china_timezone();
        china_tz.with_ymd_and_hms(year, month, day, hour, min, sec).single()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_timezone_conversion() {
        let utc_time = Utc::now();
        let china_time = TimezoneUtil::utc_to_china(utc_time);
        let back_to_utc = TimezoneUtil::china_to_utc(china_time);
        
        // 转换后应该相等（忽略纳秒差异）
        assert_eq!(utc_time.timestamp(), back_to_utc.timestamp());
    }
    
    #[test] 
    fn test_time_difference() {
        let utc_time = Utc::now();
        let china_time = TimezoneUtil::utc_to_china(utc_time);
        
        // 中国时间应该比UTC时间早8小时
        use chrono::Timelike;
        assert_eq!(china_time.hour() as i32, (utc_time.hour() as i32 + 8) % 24);
    }
}
