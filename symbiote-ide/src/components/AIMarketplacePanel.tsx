// AI Marketplace Panel - Revolutionary AI Framework Discovery and Installation
// Phase 4 UI Component for AI Framework Marketplace integration

import React, { useState, useEffect } from 'react';
// import { invoke } from '@tauri-apps/api/tauri';
import './AIMarketplacePanel.css';

interface MarketplacePackage {
  package_id: string;
  name: string;
  display_name: string;
  description: string;
  package_type: string;
  category: string;
  version: string;
  author: {
    name: string;
    verified: boolean;
  };
  ratings: {
    average_rating: number;
    total_ratings: number;
  };
  download_stats: {
    total_downloads: number;
    monthly_downloads: number;
  };
  pricing: {
    pricing_model: string;
    base_price?: number;
    currency: string;
  };
  security_info: {
    security_score: number;
    vulnerabilities: any[];
  };
  tags: string[];
  screenshots: string[];
}

interface MarketplaceState {
  searchQuery: string;
  selectedCategory: string;
  installedPackages: string[];
}

interface AIMarketplacePanelProps {
  state: MarketplaceState;
  onStateChange: (newState: MarketplaceState) => void;
}

// Mock data for development - replace with actual API calls
const mockPackages: MarketplacePackage[] = [
  {
    package_id: 'langchain-core',
    name: 'langchain',
    display_name: 'LangChain',
    description: 'Build applications with LLMs through composability',
    package_type: 'framework',
    category: 'machine-learning',
    version: '0.1.0',
    author: {
      name: 'LangChain Team',
      verified: true,
    },
    ratings: {
      average_rating: 4.8,
      total_ratings: 1250,
    },
    download_stats: {
      total_downloads: 50000,
      monthly_downloads: 8500,
    },
    pricing: {
      pricing_model: 'free',
      currency: 'USD',
    },
    security_info: {
      security_score: 9.2,
      vulnerabilities: [],
    },
    tags: ['llm', 'ai', 'framework'],
    screenshots: [],
  },
  {
    package_id: 'openai-sdk',
    name: 'openai',
    display_name: 'OpenAI SDK',
    description: 'Official OpenAI API client library',
    package_type: 'sdk',
    category: 'natural-language',
    version: '4.0.0',
    author: {
      name: 'OpenAI',
      verified: true,
    },
    ratings: {
      average_rating: 4.9,
      total_ratings: 2100,
    },
    download_stats: {
      total_downloads: 100000,
      monthly_downloads: 15000,
    },
    pricing: {
      pricing_model: 'free',
      currency: 'USD',
    },
    security_info: {
      security_score: 9.5,
      vulnerabilities: [],
    },
    tags: ['openai', 'gpt', 'api'],
    screenshots: [],
  },
];

export const AIMarketplacePanel: React.FC<AIMarketplacePanelProps> = ({
  state,
  onStateChange,
}) => {
  const [packages, setPackages] = useState<MarketplacePackage[]>([]);
  const [loading, setLoading] = useState(false);
  const [selectedPackage, setSelectedPackage] = useState<MarketplacePackage | null>(null);
  const [installing, setInstalling] = useState<Set<string>>(new Set());
  const [categories] = useState([
    'all',
    'machine-learning',
    'natural-language',
    'computer-vision',
    'code-generation',
    'testing',
    'data-science',
    'web-development',
    'mobile-development',
    'devops',
  ]);

  useEffect(() => {
    searchPackages();
  }, [state.searchQuery, state.selectedCategory]);

  const searchPackages = async () => {
    setLoading(true);
    try {
      const searchQuery = {
        query: state.searchQuery,
        filters: {
          categories: state.selectedCategory === 'all' ? [] : [state.selectedCategory],
          package_types: [],
          pricing_models: [],
          min_rating: null,
          platforms: [],
          languages: [],
          verified_only: false,
          free_only: false,
        },
        sort_by: 'Popularity',
        page: 1,
        page_size: 20,
      };

      const packages = mockPackages.filter(pkg => 
        pkg.name.toLowerCase().includes(searchQuery.query.toLowerCase()) &&
        (searchQuery.filters.categories.length === 0 || searchQuery.filters.categories.includes(pkg.category))
      );
      setPackages(packages);
    } catch (error) {
      console.error('Failed to search packages:', error);
    } finally {
      setLoading(false);
    }
  };

  const installPackage = async (packageId: string) => {
    setInstalling(prev => new Set(prev).add(packageId));
    try {
      // TODO: Replace with actual Tauri command when backend is ready
      // const config = {
      //   install_location: '/workspace/packages',
      //   create_shortcuts: true,
      //   add_to_path: true,
      //   auto_update: true,
      //   custom_settings: {},
      // };
      // const installationId = await invoke('install_marketplace_package', { packageId, config });

      console.log('Installing package:', packageId);

      // Simulate installation delay
      await new Promise(resolve => setTimeout(resolve, 2000));

      // Update installed packages
      onStateChange({
        ...state,
        installedPackages: [...state.installedPackages, packageId],
      });

      console.log('Package installation completed:', packageId);
    } catch (error) {
      console.error('Failed to install package:', error);
    } finally {
      setInstalling(prev => {
        const newSet = new Set(prev);
        newSet.delete(packageId);
        return newSet;
      });
    }
  };

  const formatNumber = (num: number): string => {
    if (num >= 1000000) return `${(num / 1000000).toFixed(1)}M`;
    if (num >= 1000) return `${(num / 1000).toFixed(1)}K`;
    return num.toString();
  };

  const getCategoryIcon = (category: string): string => {
    const icons: { [key: string]: string } = {
      'machine-learning': '🤖',
      'natural-language': '💬',
      'computer-vision': '👁️',
      'code-generation': '⚡',
      'testing': '🧪',
      'data-science': '📊',
      'web-development': '🌐',
      'mobile-development': '📱',
      'devops': '🔧',
    };
    return icons[category] || '📦';
  };

  const getSecurityBadge = (score: number): { color: string; text: string } => {
    if (score >= 90) return { color: '#27ae60', text: 'Excellent' };
    if (score >= 75) return { color: '#f39c12', text: 'Good' };
    if (score >= 60) return { color: '#e67e22', text: 'Fair' };
    return { color: '#e74c3c', text: 'Poor' };
  };

  return (
    <div className="ai-marketplace-panel">
      {/* Search and Filters */}
      <div className="marketplace-header">
        <div className="search-section">
          <input
            type="text"
            placeholder="Search AI frameworks, models, and tools..."
            value={state.searchQuery}
            onChange={(e) => onStateChange({ ...state, searchQuery: e.target.value })}
            className="search-input"
          />
          <button onClick={searchPackages} className="search-button">
            🔍 Search
          </button>
        </div>
        
        <div className="filter-section">
          <select
            value={state.selectedCategory}
            onChange={(e) => onStateChange({ ...state, selectedCategory: e.target.value })}
            className="category-select"
          >
            {categories.map(category => (
              <option key={category} value={category}>
                {getCategoryIcon(category)} {category.replace('-', ' ').toUpperCase()}
              </option>
            ))}
          </select>
        </div>
      </div>

      {/* Package Grid */}
      <div className="marketplace-content">
        {loading ? (
          <div className="loading-state">
            <div className="spinner"></div>
            <p>Searching AI Marketplace...</p>
          </div>
        ) : (
          <div className="packages-grid">
            {packages.map(pkg => (
              <div key={pkg.package_id} className="package-card">
                <div className="package-header">
                  <div className="package-info">
                    <h3 className="package-name">{pkg.display_name}</h3>
                    <p className="package-author">
                      by {pkg.author.name}
                      {pkg.author.verified && <span className="verified-badge">✓</span>}
                    </p>
                  </div>
                  <div className="package-version">v{pkg.version}</div>
                </div>

                <p className="package-description">{pkg.description}</p>

                <div className="package-tags">
                  {pkg.tags.slice(0, 3).map(tag => (
                    <span key={tag} className="tag">{tag}</span>
                  ))}
                </div>

                <div className="package-stats">
                  <div className="stat">
                    <span className="stat-icon">⭐</span>
                    <span>{pkg.ratings.average_rating.toFixed(1)}</span>
                    <span className="stat-label">({formatNumber(pkg.ratings.total_ratings)})</span>
                  </div>
                  <div className="stat">
                    <span className="stat-icon">📥</span>
                    <span>{formatNumber(pkg.download_stats.monthly_downloads)}</span>
                    <span className="stat-label">monthly</span>
                  </div>
                  <div className="stat">
                    <span 
                      className="security-badge"
                      style={{ color: getSecurityBadge(pkg.security_info.security_score).color }}
                    >
                      🛡️ {getSecurityBadge(pkg.security_info.security_score).text}
                    </span>
                  </div>
                </div>

                <div className="package-pricing">
                  {pkg.pricing.pricing_model === 'Free' ? (
                    <span className="price-free">Free</span>
                  ) : (
                    <span className="price-paid">
                      ${pkg.pricing.base_price}/{pkg.pricing.pricing_model.toLowerCase()}
                    </span>
                  )}
                </div>

                <div className="package-actions">
                  <button
                    onClick={() => setSelectedPackage(pkg)}
                    className="details-button"
                  >
                    📋 Details
                  </button>
                  {state.installedPackages.includes(pkg.package_id) ? (
                    <button className="installed-button" disabled>
                      ✅ Installed
                    </button>
                  ) : (
                    <button
                      onClick={() => installPackage(pkg.package_id)}
                      disabled={installing.has(pkg.package_id)}
                      className="install-button"
                    >
                      {installing.has(pkg.package_id) ? (
                        <>⏳ Installing...</>
                      ) : (
                        <>📦 Install</>
                      )}
                    </button>
                  )}
                </div>
              </div>
            ))}
          </div>
        )}

        {packages.length === 0 && !loading && (
          <div className="empty-state">
            <h3>No packages found</h3>
            <p>Try adjusting your search terms or filters</p>
          </div>
        )}
      </div>

      {/* Package Details Modal */}
      {selectedPackage && (
        <div className="package-details-modal">
          <div className="modal-content">
            <div className="modal-header">
              <h2>{selectedPackage.display_name}</h2>
              <button
                onClick={() => setSelectedPackage(null)}
                className="close-button"
              >
                ×
              </button>
            </div>
            
            <div className="modal-body">
              <div className="package-overview">
                <div className="overview-section">
                  <h4>Description</h4>
                  <p>{selectedPackage.description}</p>
                </div>

                <div className="overview-section">
                  <h4>Author</h4>
                  <p>
                    {selectedPackage.author.name}
                    {selectedPackage.author.verified && (
                      <span className="verified-badge">✓ Verified</span>
                    )}
                  </p>
                </div>

                <div className="overview-section">
                  <h4>Version</h4>
                  <p>v{selectedPackage.version}</p>
                </div>

                <div className="overview-section">
                  <h4>Category</h4>
                  <p>
                    {getCategoryIcon(selectedPackage.category)} {selectedPackage.category}
                  </p>
                </div>

                <div className="overview-section">
                  <h4>Security Score</h4>
                  <p style={{ color: getSecurityBadge(selectedPackage.security_info.security_score).color }}>
                    🛡️ {selectedPackage.security_info.security_score}/100 - {getSecurityBadge(selectedPackage.security_info.security_score).text}
                  </p>
                </div>

                <div className="overview-section">
                  <h4>Downloads</h4>
                  <p>
                    Total: {formatNumber(selectedPackage.download_stats.total_downloads)}<br/>
                    Monthly: {formatNumber(selectedPackage.download_stats.monthly_downloads)}
                  </p>
                </div>

                <div className="overview-section">
                  <h4>Pricing</h4>
                  <p>
                    {selectedPackage.pricing.pricing_model === 'Free' ? (
                      'Free to use'
                    ) : (
                      `$${selectedPackage.pricing.base_price} per ${selectedPackage.pricing.pricing_model.toLowerCase()}`
                    )}
                  </p>
                </div>

                <div className="overview-section">
                  <h4>Tags</h4>
                  <div className="tags-list">
                    {selectedPackage.tags.map(tag => (
                      <span key={tag} className="tag">{tag}</span>
                    ))}
                  </div>
                </div>
              </div>

              {selectedPackage.screenshots.length > 0 && (
                <div className="screenshots-section">
                  <h4>Screenshots</h4>
                  <div className="screenshots-grid">
                    {selectedPackage.screenshots.map((screenshot, index) => (
                      <img
                        key={index}
                        src={screenshot}
                        alt={`Screenshot ${index + 1}`}
                        className="screenshot"
                      />
                    ))}
                  </div>
                </div>
              )}
            </div>

            <div className="modal-footer">
              {state.installedPackages.includes(selectedPackage.package_id) ? (
                <button className="installed-button" disabled>
                  ✅ Already Installed
                </button>
              ) : (
                <button
                  onClick={() => {
                    installPackage(selectedPackage.package_id);
                    setSelectedPackage(null);
                  }}
                  disabled={installing.has(selectedPackage.package_id)}
                  className="install-button large"
                >
                  {installing.has(selectedPackage.package_id) ? (
                    <>⏳ Installing...</>
                  ) : (
                    <>📦 Install Package</>
                  )}
                </button>
              )}
            </div>
          </div>
        </div>
      )}
    </div>
  );
};

export default AIMarketplacePanel;
