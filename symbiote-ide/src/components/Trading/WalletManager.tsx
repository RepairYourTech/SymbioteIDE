/**
 * Comprehensive Wallet Management Component
 * Multi-wallet, multi-exchange, and DeFi position management
 */

import React, { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import './WalletManager.css';

interface Wallet {
  id: string;
  name: string;
  wallet_type: string;
  address: string;
  network: string;
  balances: { [key: string]: TokenBalance };
  is_active: boolean;
  security_level: string;
}

interface TokenBalance {
  symbol: string;
  name: string;
  balance: string;
  usd_value: number;
  is_native: boolean;
}

interface ExchangeAccount {
  id: string;
  exchange: string;
  balances: { [key: string]: ExchangeBalance };
  trading_fees: TradingFees;
  withdrawal_limits: WithdrawalLimits;
  is_active: boolean;
}

interface ExchangeBalance {
  asset: string;
  free: number;
  locked: number;
  total: number;
  usd_value: number;
}

interface TradingFees {
  maker_fee: number;
  taker_fee: number;
}

interface WithdrawalLimits {
  daily_limit_usd: number;
  remaining_today_usd: number;
}

interface PortfolioSummary {
  total_value_usd: number;
  asset_breakdown: { [key: string]: number };
  platform_breakdown: { [key: string]: number };
}

interface WalletManagerProps {
  enableTrading?: boolean;
}

export const WalletManager: React.FC<WalletManagerProps> = ({
  enableTrading = true
}) => {
  // State management
  const [wallets, setWallets] = useState<Wallet[]>([]);
  const [exchangeAccounts, setExchangeAccounts] = useState<ExchangeAccount[]>([]);
  const [portfolioSummary, setPortfolioSummary] = useState<PortfolioSummary | null>(null);
  
  // UI state
  const [activeTab, setActiveTab] = useState<'overview' | 'wallets' | 'exchanges'>('overview');
  const [isLoading, setIsLoading] = useState(false);
  const [showAddWallet, setShowAddWallet] = useState(false);
  const [showAddExchange, setShowAddExchange] = useState(false);

  // Form state
  const [newWalletForm, setNewWalletForm] = useState({
    name: '',
    wallet_type: 'MetaMask',
    network: 'Ethereum',
    security_level: 'Medium',
    address: ''
  });

  const [newExchangeForm, setNewExchangeForm] = useState({
    exchange: 'Binance',
    api_key: '',
    api_secret: '',
    is_sandbox: false
  });

  // Load data on component mount
  useEffect(() => {
    loadWalletData();
    const interval = setInterval(loadWalletData, 30000);
    return () => clearInterval(interval);
  }, []);

  const loadWalletData = async () => {
    try {
      setIsLoading(true);
      
      // Load mock data for development
      setWallets([
        {
          id: '1',
          name: 'Main Wallet',
          wallet_type: 'MetaMask',
          address: '0x742d35Cc6634C0532925a3b8D4C2C1C8b1e6c7F8',
          network: 'Ethereum',
          balances: {
            'ETH': { symbol: 'ETH', name: 'Ethereum', balance: '2.5', usd_value: 5000, is_native: true },
            'USDT': { symbol: 'USDT', name: 'Tether', balance: '10000', usd_value: 10000, is_native: false }
          },
          is_active: true,
          security_level: 'High'
        }
      ]);

      setExchangeAccounts([
        {
          id: '1',
          exchange: 'Binance',
          balances: {
            'BTC': { asset: 'BTC', free: 0.5, locked: 0.1, total: 0.6, usd_value: 30000 },
            'USDT': { asset: 'USDT', free: 5000, locked: 1000, total: 6000, usd_value: 6000 }
          },
          trading_fees: { maker_fee: 0.001, taker_fee: 0.001 },
          withdrawal_limits: {
            daily_limit_usd: 50000,
            remaining_today_usd: 45000
          },
          is_active: true
        }
      ]);

      setPortfolioSummary({
        total_value_usd: 51000,
        asset_breakdown: {
          'ETH': 5000,
          'BTC': 30000,
          'USDT': 16000
        },
        platform_breakdown: {
          'Wallet': 15000,
          'Binance': 36000
        }
      });
    } catch (error) {
      console.error('Failed to load wallet data:', error);
    } finally {
      setIsLoading(false);
    }
  };

  const handleCreateWallet = async () => {
    try {
      setIsLoading(true);
      const walletId = await invoke<string>('create_wallet', {
        walletConfig: newWalletForm
      });
      
      console.log('Created wallet:', walletId);
      setShowAddWallet(false);
      setNewWalletForm({
        name: '',
        wallet_type: 'MetaMask',
        network: 'Ethereum',
        security_level: 'Medium',
        address: ''
      });
      
      await loadWalletData();
    } catch (error) {
      console.error('Failed to create wallet:', error);
    } finally {
      setIsLoading(false);
    }
  };

  const formatCurrency = (amount: number) => {
    return new Intl.NumberFormat('en-US', {
      style: 'currency',
      currency: 'USD',
      minimumFractionDigits: 2,
      maximumFractionDigits: 2
    }).format(amount);
  };

  const formatNumber = (num: number, decimals: number = 4) => {
    return num.toLocaleString('en-US', {
      minimumFractionDigits: 0,
      maximumFractionDigits: decimals
    });
  };

  const renderOverview = () => (
    <div className="overview-section">
      <div className="portfolio-summary">
        <h3>Portfolio Overview</h3>
        {portfolioSummary && (
          <div className="summary-grid">
            <div className="total-value">
              <h2>{formatCurrency(portfolioSummary.total_value_usd)}</h2>
              <span>Total Portfolio Value</span>
            </div>
            
            <div className="breakdown-charts">
              <div className="asset-breakdown">
                <h4>Assets</h4>
                {Object.entries(portfolioSummary.asset_breakdown).map(([asset, value]) => (
                  <div key={asset} className="breakdown-item">
                    <span className="asset-name">{asset}</span>
                    <span className="asset-value">{formatCurrency(value)}</span>
                    <div className="asset-bar">
                      <div 
                        className="asset-fill"
                        style={{ width: `${(value / portfolioSummary.total_value_usd) * 100}%` }}
                      />
                    </div>
                  </div>
                ))}
              </div>
              
              <div className="platform-breakdown">
                <h4>Platforms</h4>
                {Object.entries(portfolioSummary.platform_breakdown).map(([platform, value]) => (
                  <div key={platform} className="breakdown-item">
                    <span className="platform-name">{platform}</span>
                    <span className="platform-value">{formatCurrency(value)}</span>
                    <div className="platform-bar">
                      <div 
                        className="platform-fill"
                        style={{ width: `${(value / portfolioSummary.total_value_usd) * 100}%` }}
                      />
                    </div>
                  </div>
                ))}
              </div>
            </div>
          </div>
        )}
      </div>
      
      <div className="quick-actions">
        <h3>Quick Actions</h3>
        <div className="action-buttons">
          <button className="action-btn primary" onClick={() => setShowAddWallet(true)}>
            Add Wallet
          </button>
          <button className="action-btn secondary" onClick={() => setShowAddExchange(true)}>
            Add Exchange
          </button>
          <button className="action-btn" onClick={loadWalletData}>
            Refresh Data
          </button>
        </div>
      </div>
    </div>
  );

  const renderWallets = () => (
    <div className="wallets-section">
      <div className="section-header">
        <h3>Wallets</h3>
        <button className="add-btn" onClick={() => setShowAddWallet(true)}>
          Add Wallet
        </button>
      </div>
      
      <div className="wallets-grid">
        {wallets.map(wallet => (
          <div key={wallet.id} className="wallet-card">
            <div className="wallet-header">
              <h4>{wallet.name}</h4>
              <span className={`wallet-type ${wallet.wallet_type.toLowerCase()}`}>
                {wallet.wallet_type}
              </span>
            </div>
            
            <div className="wallet-address">
              <span className="address-label">Address:</span>
              <span className="address-value">
                {wallet.address.slice(0, 6)}...{wallet.address.slice(-4)}
              </span>
            </div>
            
            <div className="wallet-balances">
              {Object.values(wallet.balances).map(balance => (
                <div key={balance.symbol} className="balance-item">
                  <span className="balance-symbol">{balance.symbol}</span>
                  <div className="balance-amounts">
                    <span className="balance-amount">{formatNumber(parseFloat(balance.balance))}</span>
                    <span className="balance-usd">{formatCurrency(balance.usd_value)}</span>
                  </div>
                </div>
              ))}
            </div>
            
            <div className="wallet-status">
              <span className={`status-indicator ${wallet.is_active ? 'active' : 'inactive'}`}>
                {wallet.is_active ? 'Active' : 'Inactive'}
              </span>
              <span className="security-level">{wallet.security_level} Security</span>
            </div>
          </div>
        ))}
      </div>
    </div>
  );

  const renderExchanges = () => (
    <div className="exchanges-section">
      <div className="section-header">
        <h3>Exchange Accounts</h3>
        <button className="add-btn" onClick={() => setShowAddExchange(true)}>
          Add Exchange
        </button>
      </div>
      
      <div className="exchanges-grid">
        {exchangeAccounts.map(account => (
          <div key={account.id} className="exchange-card">
            <div className="exchange-header">
              <h4>{account.exchange}</h4>
              <span className={`exchange-status ${account.is_active ? 'active' : 'inactive'}`}>
                {account.is_active ? 'Connected' : 'Disconnected'}
              </span>
            </div>
            
            <div className="exchange-balances">
              {Object.values(account.balances).map(balance => (
                <div key={balance.asset} className="balance-item">
                  <span className="balance-symbol">{balance.asset}</span>
                  <div className="balance-amounts">
                    <span className="balance-total">{formatNumber(balance.total)}</span>
                    <span className="balance-breakdown">
                      Free: {formatNumber(balance.free)} | Locked: {formatNumber(balance.locked)}
                    </span>
                    <span className="balance-usd">{formatCurrency(balance.usd_value)}</span>
                  </div>
                </div>
              ))}
            </div>
            
            <div className="exchange-info">
              <div className="trading-fees">
                <span>Maker: {(account.trading_fees.maker_fee * 100).toFixed(3)}%</span>
                <span>Taker: {(account.trading_fees.taker_fee * 100).toFixed(3)}%</span>
              </div>
              
              <div className="withdrawal-limits">
                <span>Daily: {formatCurrency(account.withdrawal_limits.remaining_today_usd)} / {formatCurrency(account.withdrawal_limits.daily_limit_usd)}</span>
              </div>
            </div>
          </div>
        ))}
      </div>
    </div>
  );

  return (
    <div className="wallet-manager">
      <div className="wallet-header">
        <h2>Wallet Manager</h2>
        {isLoading && <div className="loading-indicator">Loading...</div>}
      </div>
      
      <div className="wallet-tabs">
        {(['overview', 'wallets', 'exchanges'] as const).map(tab => (
          <button
            key={tab}
            className={`tab-btn ${activeTab === tab ? 'active' : ''}`}
            onClick={() => setActiveTab(tab)}
          >
            {tab.charAt(0).toUpperCase() + tab.slice(1)}
          </button>
        ))}
      </div>
      
      <div className="wallet-content">
        {activeTab === 'overview' && renderOverview()}
        {activeTab === 'wallets' && renderWallets()}
        {activeTab === 'exchanges' && renderExchanges()}
      </div>
      
      {/* Add Wallet Modal */}
      {showAddWallet && (
        <div className="modal-overlay" onClick={() => setShowAddWallet(false)}>
          <div className="modal-content" onClick={e => e.stopPropagation()}>
            <h3>Add New Wallet</h3>
            <form onSubmit={(e) => { e.preventDefault(); handleCreateWallet(); }}>
              <div className="form-group">
                <label>Wallet Name</label>
                <input
                  type="text"
                  value={newWalletForm.name}
                  onChange={(e) => setNewWalletForm({...newWalletForm, name: e.target.value})}
                  required
                />
              </div>
              
              <div className="form-group">
                <label>Wallet Type</label>
                <select
                  value={newWalletForm.wallet_type}
                  onChange={(e) => setNewWalletForm({...newWalletForm, wallet_type: e.target.value})}
                >
                  <option value="MetaMask">MetaMask</option>
                  <option value="TrustWallet">Trust Wallet</option>
                  <option value="Phantom">Phantom</option>
                  <option value="WatchOnly">Watch Only</option>
                </select>
              </div>
              
              <div className="form-group">
                <label>Network</label>
                <select
                  value={newWalletForm.network}
                  onChange={(e) => setNewWalletForm({...newWalletForm, network: e.target.value})}
                >
                  <option value="Ethereum">Ethereum</option>
                  <option value="Polygon">Polygon</option>
                  <option value="BSC">BSC</option>
                  <option value="Solana">Solana</option>
                </select>
              </div>
              
              {newWalletForm.wallet_type === 'WatchOnly' && (
                <div className="form-group">
                  <label>Wallet Address</label>
                  <input
                    type="text"
                    value={newWalletForm.address}
                    onChange={(e) => setNewWalletForm({...newWalletForm, address: e.target.value})}
                    placeholder="0x..."
                    required
                  />
                </div>
              )}
              
              <div className="form-actions">
                <button type="button" onClick={() => setShowAddWallet(false)}>Cancel</button>
                <button type="submit" disabled={isLoading}>Create Wallet</button>
              </div>
            </form>
          </div>
        </div>
      )}
      
      {/* Add Exchange Modal */}
      {showAddExchange && (
        <div className="modal-overlay" onClick={() => setShowAddExchange(false)}>
          <div className="modal-content" onClick={e => e.stopPropagation()}>
            <h3>Add Exchange Account</h3>
            <form onSubmit={(e) => { e.preventDefault(); }}>
              <div className="form-group">
                <label>Exchange</label>
                <select
                  value={newExchangeForm.exchange}
                  onChange={(e) => setNewExchangeForm({...newExchangeForm, exchange: e.target.value})}
                >
                  <option value="Binance">Binance</option>
                  <option value="Coinbase">Coinbase</option>
                  <option value="Kraken">Kraken</option>
                  <option value="KuCoin">KuCoin</option>
                </select>
              </div>
              
              <div className="form-group">
                <label>API Key</label>
                <input
                  type="password"
                  value={newExchangeForm.api_key}
                  onChange={(e) => setNewExchangeForm({...newExchangeForm, api_key: e.target.value})}
                  required
                />
              </div>
              
              <div className="form-group">
                <label>API Secret</label>
                <input
                  type="password"
                  value={newExchangeForm.api_secret}
                  onChange={(e) => setNewExchangeForm({...newExchangeForm, api_secret: e.target.value})}
                  required
                />
              </div>
              
              <div className="form-actions">
                <button type="button" onClick={() => setShowAddExchange(false)}>Cancel</button>
                <button type="submit" disabled={isLoading}>Add Exchange</button>
              </div>
            </form>
          </div>
        </div>
      )}
    </div>
  );
};

export default WalletManager;
