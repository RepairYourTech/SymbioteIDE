import React from 'react';
import './LoadingIndicator.css';

export const LoadingIndicator: React.FC = () => {
  return (
    <div className="loading-indicator">
      <div className="loading-dots">
        <span className="loading-dot"></span>
        <span className="loading-dot"></span>
        <span className="loading-dot"></span>
      </div>
      <span className="loading-text">Assistant is thinking...</span>
    </div>
  );
};