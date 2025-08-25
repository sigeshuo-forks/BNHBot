use crate::models::ranking::*;
use axum::{
    extract::Query,
    response::Json,
    http::StatusCode,
};
use serde::Deserialize;
use chrono::Utc;
use rust_decimal::Decimal;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct MockRankingQuery {
    pub period: Option<String>,
    pub limit: Option<u32>,
}

/// 获取Mock排名数据
pub async fn get_mock_rankings(
    Query(_query): Query<MockRankingQuery>,
) -> Result<Json<RankingResponse>, StatusCode> {
    let mock_rankings = create_mock_rankings();
    
    Ok(Json(RankingResponse {
        daily_rankings: mock_rankings.clone(),
        weekly_rankings: mock_rankings.clone(),
        monthly_rankings: mock_rankings,
        last_updated: Utc::now(),
    }))
}

/// 获取特定周期的Mock排名
pub async fn get_mock_period_rankings(
    Query(query): Query<MockRankingQuery>,
) -> Result<Json<Vec<RankingEntry>>, StatusCode> {
    let mock_rankings = create_mock_rankings();
    
    let limited_rankings = if let Some(limit) = query.limit {
        mock_rankings.into_iter().take(limit as usize).collect()
    } else {
        mock_rankings
    };
    
    Ok(Json(limited_rankings))
}

fn create_mock_rankings() -> Vec<RankingEntry> {
    let mut rankings = vec![
        RankingEntry {
            user_id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440001").unwrap(),
            user_name: "交易大神".to_string(),
            exchange_type: "Binance".to_string(),
            current_balance: Decimal::new(125000, 2), // $1,250.00
            previous_balance: Some(Decimal::new(50000, 2)), // $500.00
            change_amount: Decimal::new(75000, 2), // +$750.00
            change_percentage: Decimal::new(15000, 2), // +150%
            rank: 0, // 将在排序后设置
            is_doubled: true,
            balance_history: vec![
                BalanceHistoryPoint { date: "2025-08-20".to_string(), balance: Decimal::new(50000, 2) },
                BalanceHistoryPoint { date: "2025-08-21".to_string(), balance: Decimal::new(65000, 2) },
                BalanceHistoryPoint { date: "2025-08-22".to_string(), balance: Decimal::new(85000, 2) },
                BalanceHistoryPoint { date: "2025-08-23".to_string(), balance: Decimal::new(105000, 2) },
                BalanceHistoryPoint { date: "2025-08-24".to_string(), balance: Decimal::new(115000, 2) },
                BalanceHistoryPoint { date: "2025-08-25".to_string(), balance: Decimal::new(125000, 2) },
            ],
        },
        RankingEntry {
            user_id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440002").unwrap(),
            user_name: "稳健投资者".to_string(),
            exchange_type: "OKX".to_string(),
            current_balance: Decimal::new(98000, 2), // $980.00
            previous_balance: Some(Decimal::new(80000, 2)), // $800.00
            change_amount: Decimal::new(18000, 2), // +$180.00
            change_percentage: Decimal::new(2250, 2), // +22.5%
            rank: 0, // 将在排序后设置
            is_doubled: false,
            balance_history: vec![
                BalanceHistoryPoint { date: "2025-08-20".to_string(), balance: Decimal::new(80000, 2) },
                BalanceHistoryPoint { date: "2025-08-21".to_string(), balance: Decimal::new(82000, 2) },
                BalanceHistoryPoint { date: "2025-08-22".to_string(), balance: Decimal::new(88000, 2) },
                BalanceHistoryPoint { date: "2025-08-23".to_string(), balance: Decimal::new(92000, 2) },
                BalanceHistoryPoint { date: "2025-08-24".to_string(), balance: Decimal::new(95000, 2) },
                BalanceHistoryPoint { date: "2025-08-25".to_string(), balance: Decimal::new(98000, 2) },
            ],
        },
        RankingEntry {
            user_id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440003").unwrap(),
            user_name: "币圈新手".to_string(),
            exchange_type: "Binance".to_string(),
            current_balance: Decimal::new(75000, 2), // $750.00
            previous_balance: Some(Decimal::new(100000, 2)), // $1000.00
            change_amount: Decimal::new(-25000, 2), // -$250.00
            change_percentage: Decimal::new(-2500, 2), // -25%
            rank: 0, // 将在排序后设置
            is_doubled: false,
            balance_history: vec![
                BalanceHistoryPoint { date: "2025-08-20".to_string(), balance: Decimal::new(100000, 2) },
                BalanceHistoryPoint { date: "2025-08-21".to_string(), balance: Decimal::new(95000, 2) },
                BalanceHistoryPoint { date: "2025-08-22".to_string(), balance: Decimal::new(88000, 2) },
                BalanceHistoryPoint { date: "2025-08-23".to_string(), balance: Decimal::new(82000, 2) },
                BalanceHistoryPoint { date: "2025-08-24".to_string(), balance: Decimal::new(78000, 2) },
                BalanceHistoryPoint { date: "2025-08-25".to_string(), balance: Decimal::new(75000, 2) },
            ],
        },
        RankingEntry {
            user_id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440004").unwrap(),
            user_name: "量化高手".to_string(),
            exchange_type: "OKX".to_string(),
            current_balance: Decimal::new(68000, 2), // $680.00
            previous_balance: Some(Decimal::new(60000, 2)), // $600.00
            change_amount: Decimal::new(8000, 2), // +$80.00
            change_percentage: Decimal::new(1333, 2), // +13.33%
            rank: 0, // 将在排序后设置
            is_doubled: false,
            balance_history: vec![
                BalanceHistoryPoint { date: "2025-08-20".to_string(), balance: Decimal::new(60000, 2) },
                BalanceHistoryPoint { date: "2025-08-21".to_string(), balance: Decimal::new(61000, 2) },
                BalanceHistoryPoint { date: "2025-08-22".to_string(), balance: Decimal::new(63000, 2) },
                BalanceHistoryPoint { date: "2025-08-23".to_string(), balance: Decimal::new(65000, 2) },
                BalanceHistoryPoint { date: "2025-08-24".to_string(), balance: Decimal::new(66500, 2) },
                BalanceHistoryPoint { date: "2025-08-25".to_string(), balance: Decimal::new(68000, 2) },
            ],
        },
        RankingEntry {
            user_id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440005").unwrap(),
            user_name: "佛系持币".to_string(),
            exchange_type: "Binance".to_string(),
            current_balance: Decimal::new(52000, 2), // $520.00
            previous_balance: Some(Decimal::new(50000, 2)), // $500.00
            change_amount: Decimal::new(2000, 2), // +$20.00
            change_percentage: Decimal::new(400, 2), // +4%
            rank: 0, // 将在排序后设置
            is_doubled: false,
            balance_history: vec![
                BalanceHistoryPoint { date: "2025-08-20".to_string(), balance: Decimal::new(50000, 2) },
                BalanceHistoryPoint { date: "2025-08-21".to_string(), balance: Decimal::new(50500, 2) },
                BalanceHistoryPoint { date: "2025-08-22".to_string(), balance: Decimal::new(51000, 2) },
                BalanceHistoryPoint { date: "2025-08-23".to_string(), balance: Decimal::new(51200, 2) },
                BalanceHistoryPoint { date: "2025-08-24".to_string(), balance: Decimal::new(51800, 2) },
                BalanceHistoryPoint { date: "2025-08-25".to_string(), balance: Decimal::new(52000, 2) },
            ],
        },
        RankingEntry {
            user_id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440006").unwrap(),
            user_name: "追涨杀跌王".to_string(),
            exchange_type: "OKX".to_string(),
            current_balance: Decimal::new(35000, 2), // $350.00
            previous_balance: Some(Decimal::new(80000, 2)), // $800.00
            change_amount: Decimal::new(-45000, 2), // -$450.00
            change_percentage: Decimal::new(-5625, 2), // -56.25%
            rank: 0, // 将在排序后设置
            is_doubled: false,
            balance_history: vec![
                BalanceHistoryPoint { date: "2025-08-20".to_string(), balance: Decimal::new(80000, 2) },
                BalanceHistoryPoint { date: "2025-08-21".to_string(), balance: Decimal::new(70000, 2) },
                BalanceHistoryPoint { date: "2025-08-22".to_string(), balance: Decimal::new(55000, 2) },
                BalanceHistoryPoint { date: "2025-08-23".to_string(), balance: Decimal::new(45000, 2) },
                BalanceHistoryPoint { date: "2025-08-24".to_string(), balance: Decimal::new(40000, 2) },
                BalanceHistoryPoint { date: "2025-08-25".to_string(), balance: Decimal::new(35000, 2) },
            ],
        },
        RankingEntry {
            user_id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440007").unwrap(),
            user_name: "翻倍达人".to_string(),
            exchange_type: "Binance".to_string(),
            current_balance: Decimal::new(32000, 2), // $320.00
            previous_balance: Some(Decimal::new(15000, 2)), // $150.00
            change_amount: Decimal::new(17000, 2), // +$170.00
            change_percentage: Decimal::new(11333, 2), // +113.33%
            rank: 0, // 将在排序后设置
            is_doubled: true,
            balance_history: vec![
                BalanceHistoryPoint { date: "2025-08-20".to_string(), balance: Decimal::new(15000, 2) },
                BalanceHistoryPoint { date: "2025-08-21".to_string(), balance: Decimal::new(18000, 2) },
                BalanceHistoryPoint { date: "2025-08-22".to_string(), balance: Decimal::new(22000, 2) },
                BalanceHistoryPoint { date: "2025-08-23".to_string(), balance: Decimal::new(26000, 2) },
                BalanceHistoryPoint { date: "2025-08-24".to_string(), balance: Decimal::new(29000, 2) },
                BalanceHistoryPoint { date: "2025-08-25".to_string(), balance: Decimal::new(32000, 2) },
            ],
        },
    ];

    // 按增长比例排序（从高到低）
    rankings.sort_by(|a, b| b.change_percentage.cmp(&a.change_percentage));
    
    // 设置排名
    for (index, ranking) in rankings.iter_mut().enumerate() {
        ranking.rank = (index + 1) as u32;
    }
    
    rankings
}