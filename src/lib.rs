pub mod handlers;
pub mod middleware;
pub mod models;
pub mod services;
pub mod utils;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exchange_type_parsing() {
        use models::ExchangeType;
        use std::str::FromStr;

        assert_eq!(
            ExchangeType::from_str("binance").unwrap(),
            ExchangeType::Binance
        );
        assert_eq!(ExchangeType::from_str("okx").unwrap(), ExchangeType::Okx);
        assert_eq!(ExchangeType::from_str("weex").unwrap(), ExchangeType::Weex);

        assert!(ExchangeType::from_str("invalid").is_err());
    }

    #[test]
    fn test_exchange_type_display() {
        use models::ExchangeType;

        assert_eq!(ExchangeType::Binance.to_string(), "币安");
        assert_eq!(ExchangeType::Okx.to_string(), "欧易");
        assert_eq!(ExchangeType::Weex.to_string(), "WEEX");
    }
}
