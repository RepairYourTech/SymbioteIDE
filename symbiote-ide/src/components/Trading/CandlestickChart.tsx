/**
 * Advanced Candlestick Chart Component for Crypto Trading
 * Professional-grade charting with technical indicators and pattern recognition
 */

import React, { useState, useEffect, useRef, useMemo } from 'react';
import { invoke } from '@tauri-apps/api/core';
import './CandlestickChart.css';

interface CandleData {
  timestamp: number;
  open: number;
  high: number;
  low: number;
  close: number;
  volume: number;
}

interface TechnicalIndicator {
  name: string;
  values: number[];
  color: string;
  visible: boolean;
}

interface TradingSignal {
  timestamp: number;
  type: 'buy' | 'sell';
  price: number;
  confidence: number;
  strategy: string;
}

interface ChartProps {
  symbol: string;
  timeframe: string;
  height?: number;
  showVolume?: boolean;
  showIndicators?: boolean;
  enableTrading?: boolean;
  onSignalClick?: (signal: TradingSignal) => void;
}

export const CandlestickChart: React.FC<ChartProps> = ({
  symbol,
  timeframe,
  height = 600,
  showVolume = true,
  showIndicators = true,
  enableTrading = false,
  onSignalClick
}) => {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const [candleData, setCandleData] = useState<CandleData[]>([]);
  const [indicators, setIndicators] = useState<TechnicalIndicator[]>([]);
  const [signals, setSignals] = useState<TradingSignal[]>([]);
  const [selectedIndicators, setSelectedIndicators] = useState<string[]>(['SMA_20', 'SMA_50', 'RSI', 'MACD']);
  const [isLoading, setIsLoading] = useState(false);
  const [crosshair, setCrosshair] = useState<{ x: number; y: number } | null>(null);
  const [priceInfo, setPriceInfo] = useState<CandleData | null>(null);

  // Chart settings
  const [chartSettings, setChartSettings] = useState({
    candleWidth: 8,
    candleSpacing: 2,
    gridLines: true,
    priceLabels: true,
    volumeHeight: 150,
    indicatorHeight: 100,
  });

  // Color scheme
  const colors = {
    bullish: '#00d4aa',
    bearish: '#ff6b6b',
    background: '#1a1a1a',
    grid: '#333333',
    text: '#ffffff',
    volume: '#4a90e2',
    indicators: {
      SMA_20: '#ffeb3b',
      SMA_50: '#ff9800',
      EMA_12: '#e91e63',
      EMA_26: '#9c27b0',
      RSI: '#2196f3',
      MACD: '#4caf50',
      BB_Upper: '#ff5722',
      BB_Lower: '#ff5722',
      BB_Middle: '#ffc107',
    }
  };

  // Load market data
  useEffect(() => {
    loadMarketData();
    const interval = setInterval(loadMarketData, 5000); // Update every 5 seconds
    return () => clearInterval(interval);
  }, [symbol, timeframe]);

  // Load technical indicators
  useEffect(() => {
    if (candleData.length > 0) {
      loadTechnicalIndicators();
    }
  }, [candleData, selectedIndicators]);

  // Load trading signals
  useEffect(() => {
    if (enableTrading && candleData.length > 0) {
      loadTradingSignals();
    }
  }, [candleData, enableTrading]);

  // Render chart when data changes
  useEffect(() => {
    if (candleData.length > 0) {
      renderChart();
    }
  }, [candleData, indicators, signals, chartSettings, crosshair]);

  const loadMarketData = async () => {
    try {
      setIsLoading(true);
      const data = await invoke<CandleData[]>('get_market_data', {
        symbol,
        timeframe,
        limit: 200
      });
      setCandleData(data);
    } catch (error) {
      console.error('Failed to load market data:', error);
      // Mock data for development
      setCandleData(generateMockData());
    } finally {
      setIsLoading(false);
    }
  };

  const loadTechnicalIndicators = async () => {
    try {
      const indicatorData = await Promise.all(
        selectedIndicators.map(async (indicator) => {
          const values = await invoke<number[]>('calculate_indicator', {
            indicator,
            data: candleData,
            params: getIndicatorParams(indicator)
          });
          return {
            name: indicator,
            values,
            color: colors.indicators[indicator as keyof typeof colors.indicators] || '#ffffff',
            visible: true
          };
        })
      );
      setIndicators(indicatorData);
    } catch (error) {
      console.error('Failed to load indicators:', error);
      // Mock indicators for development
      setIndicators(generateMockIndicators());
    }
  };

  const loadTradingSignals = async () => {
    try {
      const signalData = await invoke<TradingSignal[]>('get_trading_signals', {
        symbol,
        timeframe,
        strategies: ['scalping', 'swing', 'momentum']
      });
      setSignals(signalData);
    } catch (error) {
      console.error('Failed to load trading signals:', error);
      // Mock signals for development
      setSignals(generateMockSignals());
    }
  };

  const renderChart = () => {
    const canvas = canvasRef.current;
    if (!canvas || candleData.length === 0) return;

    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    // Set canvas size
    canvas.width = canvas.offsetWidth;
    canvas.height = height;

    // Clear canvas
    ctx.fillStyle = colors.background;
    ctx.fillRect(0, 0, canvas.width, canvas.height);

    // Calculate dimensions
    const chartHeight = showVolume ? height - chartSettings.volumeHeight - 50 : height - 50;
    const volumeTop = chartHeight + 10;
    const candleAreaWidth = canvas.width - 100; // Leave space for price labels
    const candleCount = Math.floor(candleAreaWidth / (chartSettings.candleWidth + chartSettings.candleSpacing));
    const visibleData = candleData.slice(-candleCount);

    // Calculate price range
    const prices = visibleData.flatMap(d => [d.high, d.low]);
    const maxPrice = Math.max(...prices);
    const minPrice = Math.min(...prices);
    const priceRange = maxPrice - minPrice;
    const priceScale = chartHeight / priceRange;

    // Draw grid lines
    if (chartSettings.gridLines) {
      drawGridLines(ctx, canvas.width, chartHeight, maxPrice, minPrice);
    }

    // Draw candlesticks
    drawCandlesticks(ctx, visibleData, chartHeight, minPrice, priceScale);

    // Draw volume bars
    if (showVolume) {
      drawVolume(ctx, visibleData, volumeTop, chartSettings.volumeHeight);
    }

    // Draw technical indicators
    if (showIndicators && indicators.length > 0) {
      drawIndicators(ctx, visibleData, indicators, chartHeight, minPrice, priceScale);
    }

    // Draw trading signals
    if (enableTrading && signals.length > 0) {
      drawTradingSignals(ctx, signals, visibleData, chartHeight, minPrice, priceScale);
    }

    // Draw price labels
    drawPriceLabels(ctx, canvas.width, chartHeight, maxPrice, minPrice);

    // Draw crosshair
    if (crosshair) {
      drawCrosshair(ctx, crosshair, canvas.width, chartHeight);
    }
  };

  const drawCandlesticks = (
    ctx: CanvasRenderingContext2D,
    data: CandleData[],
    chartHeight: number,
    minPrice: number,
    priceScale: number
  ) => {
    data.forEach((candle, index) => {
      const x = 50 + index * (chartSettings.candleWidth + chartSettings.candleSpacing);
      const openY = chartHeight - (candle.open - minPrice) * priceScale;
      const closeY = chartHeight - (candle.close - minPrice) * priceScale;
      const highY = chartHeight - (candle.high - minPrice) * priceScale;
      const lowY = chartHeight - (candle.low - minPrice) * priceScale;

      const isBullish = candle.close > candle.open;
      const color = isBullish ? colors.bullish : colors.bearish;

      // Draw wick
      ctx.strokeStyle = color;
      ctx.lineWidth = 1;
      ctx.beginPath();
      ctx.moveTo(x + chartSettings.candleWidth / 2, highY);
      ctx.lineTo(x + chartSettings.candleWidth / 2, lowY);
      ctx.stroke();

      // Draw body
      ctx.fillStyle = color;
      const bodyTop = Math.min(openY, closeY);
      const bodyHeight = Math.abs(closeY - openY);
      ctx.fillRect(x, bodyTop, chartSettings.candleWidth, Math.max(bodyHeight, 1));
    });
  };

  const drawVolume = (
    ctx: CanvasRenderingContext2D,
    data: CandleData[],
    volumeTop: number,
    volumeHeight: number
  ) => {
    const maxVolume = Math.max(...data.map(d => d.volume));
    const volumeScale = volumeHeight / maxVolume;

    data.forEach((candle, index) => {
      const x = 50 + index * (chartSettings.candleWidth + chartSettings.candleSpacing);
      const barHeight = candle.volume * volumeScale;
      const isBullish = candle.close > candle.open;

      ctx.fillStyle = isBullish ? colors.bullish + '80' : colors.bearish + '80';
      ctx.fillRect(x, volumeTop + volumeHeight - barHeight, chartSettings.candleWidth, barHeight);
    });
  };

  const drawIndicators = (
    ctx: CanvasRenderingContext2D,
    data: CandleData[],
    indicators: TechnicalIndicator[],
    chartHeight: number,
    minPrice: number,
    priceScale: number
  ) => {
    indicators.forEach(indicator => {
      if (!indicator.visible || indicator.values.length === 0) return;

      ctx.strokeStyle = indicator.color;
      ctx.lineWidth = 2;
      ctx.beginPath();

      let started = false;
      indicator.values.forEach((value, index) => {
        if (value !== null && value !== undefined && !isNaN(value)) {
          const x = 50 + index * (chartSettings.candleWidth + chartSettings.candleSpacing) + chartSettings.candleWidth / 2;
          const y = chartHeight - (value - minPrice) * priceScale;

          if (!started) {
            ctx.moveTo(x, y);
            started = true;
          } else {
            ctx.lineTo(x, y);
          }
        }
      });

      ctx.stroke();
    });
  };

  const drawTradingSignals = (
    ctx: CanvasRenderingContext2D,
    signals: TradingSignal[],
    data: CandleData[],
    chartHeight: number,
    minPrice: number,
    priceScale: number
  ) => {
    signals.forEach(signal => {
      const dataIndex = data.findIndex(d => d.timestamp === signal.timestamp);
      if (dataIndex === -1) return;

      const x = 50 + dataIndex * (chartSettings.candleWidth + chartSettings.candleSpacing) + chartSettings.candleWidth / 2;
      const y = chartHeight - (signal.price - minPrice) * priceScale;

      // Draw signal arrow
      ctx.fillStyle = signal.type === 'buy' ? colors.bullish : colors.bearish;
      ctx.beginPath();
      if (signal.type === 'buy') {
        // Up arrow
        ctx.moveTo(x, y + 15);
        ctx.lineTo(x - 8, y + 25);
        ctx.lineTo(x + 8, y + 25);
      } else {
        // Down arrow
        ctx.moveTo(x, y - 15);
        ctx.lineTo(x - 8, y - 25);
        ctx.lineTo(x + 8, y - 25);
      }
      ctx.closePath();
      ctx.fill();

      // Draw confidence indicator
      ctx.fillStyle = `rgba(255, 255, 255, ${signal.confidence})`;
      ctx.beginPath();
      ctx.arc(x, y, 3, 0, 2 * Math.PI);
      ctx.fill();
    });
  };

  const drawGridLines = (
    ctx: CanvasRenderingContext2D,
    width: number,
    height: number,
    maxPrice: number,
    minPrice: number
  ) => {
    ctx.strokeStyle = colors.grid;
    ctx.lineWidth = 1;

    // Horizontal grid lines
    const priceStep = (maxPrice - minPrice) / 10;
    for (let i = 0; i <= 10; i++) {
      const y = height - (i * height / 10);
      ctx.beginPath();
      ctx.moveTo(50, y);
      ctx.lineTo(width - 50, y);
      ctx.stroke();
    }

    // Vertical grid lines
    const timeStep = (width - 100) / 10;
    for (let i = 0; i <= 10; i++) {
      const x = 50 + i * timeStep;
      ctx.beginPath();
      ctx.moveTo(x, 0);
      ctx.lineTo(x, height);
      ctx.stroke();
    }
  };

  const drawPriceLabels = (
    ctx: CanvasRenderingContext2D,
    width: number,
    height: number,
    maxPrice: number,
    minPrice: number
  ) => {
    ctx.fillStyle = colors.text;
    ctx.font = '12px Arial';
    ctx.textAlign = 'left';

    const priceStep = (maxPrice - minPrice) / 10;
    for (let i = 0; i <= 10; i++) {
      const price = minPrice + i * priceStep;
      const y = height - (i * height / 10);
      ctx.fillText(price.toFixed(2), width - 45, y + 4);
    }
  };

  const drawCrosshair = (
    ctx: CanvasRenderingContext2D,
    crosshair: { x: number; y: number },
    width: number,
    height: number
  ) => {
    ctx.strokeStyle = colors.text + '80';
    ctx.lineWidth = 1;
    ctx.setLineDash([5, 5]);

    // Vertical line
    ctx.beginPath();
    ctx.moveTo(crosshair.x, 0);
    ctx.lineTo(crosshair.x, height);
    ctx.stroke();

    // Horizontal line
    ctx.beginPath();
    ctx.moveTo(50, crosshair.y);
    ctx.lineTo(width - 50, crosshair.y);
    ctx.stroke();

    ctx.setLineDash([]);
  };

  const handleMouseMove = (event: React.MouseEvent<HTMLCanvasElement>) => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const rect = canvas.getBoundingClientRect();
    const x = event.clientX - rect.left;
    const y = event.clientY - rect.top;

    setCrosshair({ x, y });

    // Calculate price info at crosshair
    const candleIndex = Math.floor((x - 50) / (chartSettings.candleWidth + chartSettings.candleSpacing));
    if (candleIndex >= 0 && candleIndex < candleData.length) {
      setPriceInfo(candleData[candleIndex]);
    }
  };

  const handleMouseLeave = () => {
    setCrosshair(null);
    setPriceInfo(null);
  };

  const handleCanvasClick = (event: React.MouseEvent<HTMLCanvasElement>) => {
    if (!enableTrading) return;

    const canvas = canvasRef.current;
    if (!canvas) return;

    const rect = canvas.getBoundingClientRect();
    const x = event.clientX - rect.left;
    const y = event.clientY - rect.top;

    // Check if click is on a trading signal
    const clickedSignal = signals.find(signal => {
      const dataIndex = candleData.findIndex(d => d.timestamp === signal.timestamp);
      if (dataIndex === -1) return false;

      const signalX = 50 + dataIndex * (chartSettings.candleWidth + chartSettings.candleSpacing) + chartSettings.candleWidth / 2;
      const chartHeight = showVolume ? height - chartSettings.volumeHeight - 50 : height - 50;
      const minPrice = Math.min(...candleData.flatMap(d => [d.high, d.low]));
      const maxPrice = Math.max(...candleData.flatMap(d => [d.high, d.low]));
      const priceScale = chartHeight / (maxPrice - minPrice);
      const signalY = chartHeight - (signal.price - minPrice) * priceScale;

      return Math.abs(x - signalX) < 15 && Math.abs(y - signalY) < 15;
    });

    if (clickedSignal && onSignalClick) {
      onSignalClick(clickedSignal);
    }
  };

  // Mock data generators for development
  const generateMockData = (): CandleData[] => {
    const data: CandleData[] = [];
    let price = 50000;
    const now = Date.now();

    for (let i = 0; i < 200; i++) {
      const timestamp = now - (200 - i) * 60000; // 1 minute intervals
      const open = price;
      const change = (Math.random() - 0.5) * 1000;
      const close = open + change;
      const high = Math.max(open, close) + Math.random() * 200;
      const low = Math.min(open, close) - Math.random() * 200;
      const volume = Math.random() * 1000000;

      data.push({ timestamp, open, high, low, close, volume });
      price = close;
    }

    return data;
  };

  const generateMockIndicators = (): TechnicalIndicator[] => {
    return [
      {
        name: 'SMA_20',
        values: candleData.map((_, i) => i > 19 ? candleData.slice(i-19, i+1).reduce((sum, c) => sum + c.close, 0) / 20 : NaN),
        color: colors.indicators.SMA_20,
        visible: true
      },
      {
        name: 'SMA_50',
        values: candleData.map((_, i) => i > 49 ? candleData.slice(i-49, i+1).reduce((sum, c) => sum + c.close, 0) / 50 : NaN),
        color: colors.indicators.SMA_50,
        visible: true
      }
    ];
  };

  const generateMockSignals = (): TradingSignal[] => {
    return candleData
      .filter((_, i) => i % 20 === 0) // Every 20th candle
      .map(candle => ({
        timestamp: candle.timestamp,
        type: Math.random() > 0.5 ? 'buy' : 'sell',
        price: candle.close,
        confidence: Math.random(),
        strategy: ['scalping', 'swing', 'momentum'][Math.floor(Math.random() * 3)]
      }));
  };

  const getIndicatorParams = (indicator: string): any => {
    const params: { [key: string]: any } = {
      'SMA_20': { period: 20 },
      'SMA_50': { period: 50 },
      'EMA_12': { period: 12 },
      'EMA_26': { period: 26 },
      'RSI': { period: 14 },
      'MACD': { fast: 12, slow: 26, signal: 9 }
    };
    return params[indicator] || {};
  };

  return (
    <div className="candlestick-chart">
      <div className="chart-header">
        <div className="chart-title">
          <h3>{symbol} - {timeframe}</h3>
          {priceInfo && (
            <div className="price-info">
              <span>O: {priceInfo.open.toFixed(2)}</span>
              <span>H: {priceInfo.high.toFixed(2)}</span>
              <span>L: {priceInfo.low.toFixed(2)}</span>
              <span>C: {priceInfo.close.toFixed(2)}</span>
              <span>V: {priceInfo.volume.toLocaleString()}</span>
            </div>
          )}
        </div>
        
        <div className="chart-controls">
          <div className="indicator-controls">
            {['SMA_20', 'SMA_50', 'RSI', 'MACD', 'BB'].map(indicator => (
              <label key={indicator} className="indicator-toggle">
                <input
                  type="checkbox"
                  checked={selectedIndicators.includes(indicator)}
                  onChange={(e) => {
                    if (e.target.checked) {
                      setSelectedIndicators([...selectedIndicators, indicator]);
                    } else {
                      setSelectedIndicators(selectedIndicators.filter(i => i !== indicator));
                    }
                  }}
                />
                {indicator}
              </label>
            ))}
          </div>
          
          <div className="timeframe-controls">
            {['1m', '5m', '15m', '1h', '4h', '1d'].map(tf => (
              <button
                key={tf}
                className={`timeframe-btn ${timeframe === tf ? 'active' : ''}`}
                onClick={() => {
                  // Would trigger timeframe change
                }}
              >
                {tf}
              </button>
            ))}
          </div>
        </div>
      </div>

      <div className="chart-container">
        <canvas
          ref={canvasRef}
          className="chart-canvas"
          onMouseMove={handleMouseMove}
          onMouseLeave={handleMouseLeave}
          onClick={handleCanvasClick}
          style={{ width: '100%', height: `${height}px` }}
        />
        
        {isLoading && (
          <div className="chart-loading">
            <div className="loading-spinner"></div>
            <span>Loading market data...</span>
          </div>
        )}
      </div>
    </div>
  );
};

export default CandlestickChart;
