//! # Symbiote Trader
//!
//! AI-powered cryptocurrency trading system that democratizes algorithmic trading through:
//! - Natural language trading: "I put $100 in Coinbase, make me money"
//! - Multi-exchange support: Binance, Coinbase, Kraken, KuCoin
//! - DeFi integration: Uniswap, Aave, Compound protocols
//! - Wallet integration: MetaMask, WalletConnect, Ledger, Trezor
//! - AI strategy generation and optimization
//! - Real-time risk management and monitoring
//! - Professional trading tools and analytics

#![deny(missing_docs)]
#![warn(clippy::all)]

// Core modules
pub mod ai_trading;
pub mod exchanges;
pub mod defi;
pub mod wallets;
pub mod strategies;
pub mod risk_management;
pub mod portfolio;
pub mod analytics;
pub mod ui;
pub mod server;
pub mod types;

// Re-export main types
pub use ai_trading::{AiTradingEngine, StrategyGenerator, MarketAnalyzer};
pub use exchanges::{ExchangeManager, UnifiedExchangeApi, ExchangeConnector};
pub use defi::{DeFiManager, DeFiProtocol, LiquidityManager};
pub use wallets::{WalletManager, WalletConnector, HardwareWallet};
pub use strategies::{TradingStrategy, StrategyEngine, BacktestingEngine};
pub use risk_management::{RiskManager, RiskAssessment, PortfolioRisk};
pub use portfolio::{Portfolio, PortfolioTracker, PerformanceAnalyzer};
pub use analytics::{MarketAnalytics, TradingAnalytics, RealtimeData};
pub use ui::{TradingUI, TradingDashboard, StrategyBuilder};
pub use server::{TradingServer, WebSocketHandler, ApiRouter};
pub use types::{TradingConfig, TradeResult, MarketData};

// Core dependencies
use symbiote_core::{SymbioteResult, SymbioteError, Service};
use symbiote_ai::AIProviderManager;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Main trading system that orchestrates all trading functionality
pub struct TradingSystem {
    /// AI-powered trading engine
    pub ai_engine: Arc<AiTradingEngine>,
    
    /// Multi-exchange management
    pub exchange_manager: Arc<ExchangeManager>,
    
    /// DeFi protocol integration
    pub defi_manager: Arc<DeFiManager>,
    
    /// Wallet management and integration
    pub wallet_manager: Arc<WalletManager>,
    
    /// Trading strategy engine
    pub strategy_engine: Arc<StrategyEngine>,
    
    /// Risk management system
    pub risk_manager: Arc<RiskManager>,
    
    /// Portfolio tracking and analytics
    pub portfolio_tracker: Arc<PortfolioTracker>,
    
    /// Market analytics and data
    pub market_analytics: Arc<MarketAnalytics>,
    
    /// Trading UI system
    pub trading_ui: Arc<TradingUI>,
    
    /// AI provider integration
    pub ai_provider: Arc<AIProviderManager>,
    
    /// Configuration
    config: Arc<RwLock<TradingConfig>>,
}

/// Configuration for the trading system
#[derive(Debug, Clone)]
pub struct TradingConfig {
    /// Enable AI-powered features
    pub enable_ai_features: bool,
    
    /// Default risk tolerance (0.0 to 1.0)
    pub default_risk_tolerance: f64,
    
    /// Maximum position size as percentage of portfolio
    pub max_position_size: f64,
    
    /// Enable DeFi features
    pub enable_defi: bool,
    
    /// Database connection string
    pub database_url: String,
    
    /// Exchange configurations
    pub exchanges: ExchangeConfigs,
    
    /// Server configuration
    pub server: ServerConfig,
}

/// Exchange configurations
#[derive(Debug, Clone)]
pub struct ExchangeConfigs {
    /// Binance configuration
    pub binance: Option<ExchangeConfig>,
    
    /// Coinbase configuration
    pub coinbase: Option<ExchangeConfig>,
    
    /// Kraken configuration
    pub kraken: Option<ExchangeConfig>,
    
    /// KuCoin configuration
    pub kucoin: Option<ExchangeConfig>,
}

/// Individual exchange configuration
#[derive(Debug, Clone)]
pub struct ExchangeConfig {
    /// API key (stored securely in vault)
    pub api_key_id: String,
    
    /// Enable trading (vs read-only)
    pub enable_trading: bool,
    
    /// Enable testnet/sandbox mode
    pub testnet: bool,
}

/// Server configuration
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Server host
    pub host: String,
    
    /// Server port
    pub port: u16,
    
    /// Enable WebSocket support
    pub enable_websocket: bool,
}

impl Default for TradingConfig {
    fn default() -> Self {
        Self {
            enable_ai_features: true,
            default_risk_tolerance: 0.5, // Medium risk
            max_position_size: 0.1, // 10% max position size
            enable_defi: true,
            database_url: "postgresql://localhost/symbiote_trader".to_string(),
            exchanges: ExchangeConfigs {
                binance: None,
                coinbase: None,
                kraken: None,
                kucoin: None,
            },
            server: ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 8082,
                enable_websocket: true,
            },
        }
    }
}

impl TradingSystem {
    /// Create a new trading system instance
    pub async fn new(config: TradingConfig, ai_provider: Arc<AIProviderManager>) -> SymbioteResult<Self> {
        let config = Arc::new(RwLock::new(config));
        
        // Initialize core components
        let ai_engine = Arc::new(AiTradingEngine::new(config.clone(), ai_provider.clone()).await?);
        let exchange_manager = Arc::new(ExchangeManager::new(config.clone()).await?);
        let defi_manager = Arc::new(DeFiManager::new(config.clone()).await?);
        let wallet_manager = Arc::new(WalletManager::new(config.clone()).await?);
        let strategy_engine = Arc::new(StrategyEngine::new(config.clone()).await?);
        let risk_manager = Arc::new(RiskManager::new(config.clone()).await?);
        let portfolio_tracker = Arc::new(PortfolioTracker::new(config.clone()).await?);
        let market_analytics = Arc::new(MarketAnalytics::new(config.clone()).await?);
        let trading_ui = Arc::new(TradingUI::new(config.clone()).await?);
        
        Ok(Self {
            ai_engine,
            exchange_manager,
            defi_manager,
            wallet_manager,
            strategy_engine,
            risk_manager,
            portfolio_tracker,
            market_analytics,
            trading_ui,
            ai_provider,
            config,
        })
    }
    
    /// Start the trading system
    pub async fn start(&self) -> SymbioteResult<()> {
        tracing::info!("Starting Symbiote Trading System");
        
        // Initialize all components
        self.ai_engine.initialize().await?;
        self.exchange_manager.initialize().await?;
        self.defi_manager.initialize().await?;
        self.wallet_manager.initialize().await?;
        self.strategy_engine.initialize().await?;
        self.risk_manager.initialize().await?;
        self.portfolio_tracker.initialize().await?;
        self.market_analytics.initialize().await?;
        self.trading_ui.initialize().await?;
        
        tracing::info!("Symbiote Trading System started successfully");
        Ok(())
    }
    
    /// Stop the trading system
    pub async fn stop(&self) -> SymbioteResult<()> {
        tracing::info!("Stopping Symbiote Trading System");
        
        // Shutdown all components in reverse order
        self.trading_ui.shutdown().await?;
        self.market_analytics.shutdown().await?;
        self.portfolio_tracker.shutdown().await?;
        self.risk_manager.shutdown().await?;
        self.strategy_engine.shutdown().await?;
        self.wallet_manager.shutdown().await?;
        self.defi_manager.shutdown().await?;
        self.exchange_manager.shutdown().await?;
        self.ai_engine.shutdown().await?;
        
        tracing::info!("Symbiote Trading System stopped successfully");
        Ok(())
    }
    
    /// Process natural language trading request
    pub async fn process_trading_request(&self, request: &str, user_id: &str) -> SymbioteResult<TradeResult> {
        self.ai_engine.process_natural_language_request(request, user_id).await
    }
    
    /// Get current portfolio status
    pub async fn get_portfolio_status(&self, user_id: &str) -> SymbioteResult<Portfolio> {
        self.portfolio_tracker.get_portfolio(user_id).await
    }
    
    /// Get the current configuration
    pub async fn get_config(&self) -> TradingConfig {
        self.config.read().await.clone()
    }
    
    /// Update the configuration
    pub async fn update_config(&self, new_config: TradingConfig) -> SymbioteResult<()> {
        let mut config = self.config.write().await;
        *config = new_config;
        
        // Notify all components of config change
        self.ai_engine.on_config_changed(&*config).await?;
        self.exchange_manager.on_config_changed(&*config).await?;
        self.defi_manager.on_config_changed(&*config).await?;
        self.wallet_manager.on_config_changed(&*config).await?;
        self.strategy_engine.on_config_changed(&*config).await?;
        self.risk_manager.on_config_changed(&*config).await?;
        self.portfolio_tracker.on_config_changed(&*config).await?;
        self.market_analytics.on_config_changed(&*config).await?;
        self.trading_ui.on_config_changed(&*config).await?;
        
        Ok(())
    }
}

#[async_trait::async_trait]
impl Service for TradingSystem {
    fn name(&self) -> &'static str {
        "trading_system"
    }
    
    fn dependencies(&self) -> Vec<&'static str> {
        vec!["ai_provider", "vault", "storage"]
    }
    
    async fn initialize(&mut self) -> SymbioteResult<()> {
        self.start().await
    }
    
    async fn shutdown(&mut self) -> SymbioteResult<()> {
        self.stop().await
    }
}

/// Initialize the trading system with default configuration
pub async fn initialize_trading_system(ai_provider: Arc<AIProviderManager>) -> SymbioteResult<TradingSystem> {
    let config = TradingConfig::default();
    TradingSystem::new(config, ai_provider).await
}

/// Initialize the trading system with custom configuration
pub async fn initialize_trading_system_with_config(
    config: TradingConfig,
    ai_provider: Arc<AIProviderManager>,
) -> SymbioteResult<TradingSystem> {
    TradingSystem::new(config, ai_provider).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use symbiote_ai::AIProviderManager;

    #[tokio::test]
    async fn test_trading_system_creation() {
        let ai_provider = Arc::new(AIProviderManager::new().await.unwrap());
        let config = TradingConfig::default();
        let trading_system = TradingSystem::new(config, ai_provider).await;
        assert!(trading_system.is_ok());
    }

    #[tokio::test]
    async fn test_trading_system_lifecycle() {
        let ai_provider = Arc::new(AIProviderManager::new().await.unwrap());
        let config = TradingConfig::default();
        let trading_system = TradingSystem::new(config, ai_provider).await.unwrap();
        
        let start_result = trading_system.start().await;
        assert!(start_result.is_ok());
        
        let stop_result = trading_system.stop().await;
        assert!(stop_result.is_ok());
    }
}
