// Plan Generator - Interactive AI-powered plan creation
import React, { useState } from 'react';
import { PASystemService } from '../../services/PASystemService';
import { PlanGenerationRequest, TaskPriority } from './types';
import './PlanGenerator.css';

interface PlanGeneratorProps {
  onPlanGenerated?: (plan: any) => void;
  onError?: (error: string) => void;
}

export const PlanGenerator: React.FC<PlanGeneratorProps> = ({ onPlanGenerated, onError }) => {
  const [isGenerating, setIsGenerating] = useState(false);
  const [formData, setFormData] = useState({
    title: '',
    description: '',
    priority: TaskPriority.Medium,
    deadline: '',
    context: ''
  });

  const paService = PASystemService.getInstance();

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    
    if (!formData.title.trim() || !formData.description.trim()) {
      onError?.('Title and description are required');
      return;
    }

    setIsGenerating(true);

    try {
      const request: PlanGenerationRequest = {
        title: formData.title,
        description: formData.description,
        priority: formData.priority,
        deadline: formData.deadline,
        context: formData.context || undefined
      };

      const generatedPlan = await paService.generatePlan(request as any);
      onPlanGenerated?.(generatedPlan);
      
      // Reset form
      setFormData({
        title: '',
        description: '',
        priority: TaskPriority.Medium,
        deadline: '',
        context: ''
      });
    } catch (error) {
      onError?.(error instanceof Error ? error.message : 'Failed to generate plan');
    } finally {
      setIsGenerating(false);
    }
  };

  const handleInputChange = (field: string, value: any) => {
    setFormData(prev => ({ ...prev, [field]: value }));
  };

  return (
    <div className="plan-generator">
      <div className="plan-generator-header">
        <h2>🧠 AI Plan Generator</h2>
        <p>Describe your goal and let AI create a comprehensive action plan</p>
      </div>

      <form onSubmit={handleSubmit} className="plan-generator-form">
        <div className="form-group">
          <label htmlFor="title">Plan Title *</label>
          <input
            id="title"
            type="text"
            value={formData.title}
            onChange={(e) => handleInputChange('title', e.target.value)}
            placeholder="e.g., Build a React dashboard component"
            disabled={isGenerating}
            required
          />
        </div>

        <div className="form-group">
          <label htmlFor="description">Description *</label>
          <textarea
            id="description"
            value={formData.description}
            onChange={(e) => handleInputChange('description', e.target.value)}
            placeholder="Describe what you want to accomplish in detail..."
            rows={4}
            disabled={isGenerating}
            required
          />
        </div>

        <div className="form-row">
          <div className="form-group">
            <label htmlFor="priority">Priority</label>
            <select
              id="priority"
              value={formData.priority}
              onChange={(e) => handleInputChange('priority', e.target.value as TaskPriority)}
              disabled={isGenerating}
            >
              <option value={TaskPriority.Low}>🟢 Low</option>
              <option value={TaskPriority.Medium}>🟡 Medium</option>
              <option value={TaskPriority.High}>🟠 High</option>
              <option value={TaskPriority.Critical}>🔴 Critical</option>
            </select>
          </div>

          <div className="form-group">
            <label htmlFor="deadline">Deadline (Optional)</label>
            <input
              id="deadline"
              type="datetime-local"
              value={formData.deadline}
              onChange={(e) => handleInputChange('deadline', e.target.value)}
              disabled={isGenerating}
            />
          </div>
        </div>

        <div className="form-group">
          <label htmlFor="context">Additional Context (Optional)</label>
          <textarea
            id="context"
            value={formData.context}
            onChange={(e) => handleInputChange('context', e.target.value)}
            placeholder="Any additional context, constraints, or preferences..."
            rows={2}
            disabled={isGenerating}
          />
        </div>

        <div className="form-actions">
          <button
            type="submit"
            disabled={isGenerating || !formData.title.trim() || !formData.description.trim()}
            className="generate-btn"
          >
            {isGenerating ? (
              <>
                <span className="spinner"></span>
                Generating Plan...
              </>
            ) : (
              <>
                ✨ Generate AI Plan
              </>
            )}
          </button>
        </div>
      </form>

      {isGenerating && (
        <div className="generation-status">
          <div className="status-indicator">
            <div className="pulse-dot"></div>
            <span>AI is analyzing your request and creating an optimal plan...</span>
          </div>
        </div>
      )}
    </div>
  );
};

export default PlanGenerator;
