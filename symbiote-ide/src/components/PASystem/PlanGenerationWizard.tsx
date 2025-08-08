// Plan Generation Wizard - AI-powered plan creation interface
import React, { useState } from 'react';
import { PAAgent, PlanGenerationRequest, TaskPriority, PlanningStrategy } from './types';
import './PlanGenerationWizard.css';

interface PlanGenerationWizardProps {
  onGeneratePlan: (request: PlanGenerationRequest) => void;
  paAgent: PAAgent | null;
}

export const PlanGenerationWizard: React.FC<PlanGenerationWizardProps> = ({ 
  onGeneratePlan, 
  paAgent 
}) => {
  const [currentStep, setCurrentStep] = useState<'describe' | 'configure' | 'review'>('describe');
  const [isGenerating, setIsGenerating] = useState(false);
  
  // Form state
  const [formData, setFormData] = useState<PlanGenerationRequest>({
    title: '',
    description: '',
    priority: TaskPriority.Medium,
    deadline: undefined,
    context: '',
    preferred_strategy: PlanningStrategy.Hybrid,
  });
  
  // Validation
  const [errors, setErrors] = useState<Record<string, string>>({});
  
  const updateFormData = (field: keyof PlanGenerationRequest, value: any) => {
    setFormData(prev => ({ ...prev, [field]: value }));
    // Clear error when user starts typing
    if (errors[field]) {
      setErrors(prev => ({ ...prev, [field]: '' }));
    }
  };
  
  const validateStep = (step: string): boolean => {
    const newErrors: Record<string, string> = {};
    
    if (step === 'describe') {
      if (!formData.title.trim()) {
        newErrors.title = 'Title is required';
      }
      if (!formData.description.trim()) {
        newErrors.description = 'Description is required';
      }
      if (formData.description.trim().length < 20) {
        newErrors.description = 'Description should be at least 20 characters';
      }
    }
    
    setErrors(newErrors);
    return Object.keys(newErrors).length === 0;
  };
  
  const handleNext = () => {
    if (validateStep(currentStep)) {
      if (currentStep === 'describe') {
        setCurrentStep('configure');
      } else if (currentStep === 'configure') {
        setCurrentStep('review');
      }
    }
  };
  
  const handleBack = () => {
    if (currentStep === 'configure') {
      setCurrentStep('describe');
    } else if (currentStep === 'review') {
      setCurrentStep('configure');
    }
  };
  
  const handleGenerate = async () => {
    if (!validateStep('describe')) return;
    
    setIsGenerating(true);
    try {
      await onGeneratePlan(formData);
    } finally {
      setIsGenerating(false);
    }
  };
  
  return (
    <div className="plan-generation-wizard">
      {/* Header */}
      <div className="wizard-header">
        <h1>🧠 AI Plan Generator</h1>
        <p>Describe your goals and let the PA Agent create an intelligent plan</p>
        
        {/* Progress Steps */}
        <div className="wizard-steps">
          <div className={`step ${currentStep === 'describe' ? 'active' : 'completed'}`}>
            <div className="step-number">1</div>
            <span>Describe</span>
          </div>
          <div className={`step ${currentStep === 'configure' ? 'active' : currentStep === 'review' ? 'completed' : ''}`}>
            <div className="step-number">2</div>
            <span>Configure</span>
          </div>
          <div className={`step ${currentStep === 'review' ? 'active' : ''}`}>
            <div className="step-number">3</div>
            <span>Review</span>
          </div>
        </div>
      </div>
      
      {/* Step Content */}
      <div className="wizard-content">
        {currentStep === 'describe' && (
          <DescribeStep 
            formData={formData}
            errors={errors}
            onUpdate={updateFormData}
          />
        )}
        
        {currentStep === 'configure' && (
          <ConfigureStep 
            formData={formData}
            errors={errors}
            onUpdate={updateFormData}
          />
        )}
        
        {currentStep === 'review' && (
          <ReviewStep 
            formData={formData}
            paAgent={paAgent}
          />
        )}
      </div>
      
      {/* Navigation */}
      <div className="wizard-navigation">
        {currentStep !== 'describe' && (
          <button 
            className="btn-secondary"
            onClick={handleBack}
            disabled={isGenerating}
          >
            ← Back
          </button>
        )}
        
        <div className="nav-spacer" />
        
        {currentStep !== 'review' ? (
          <button 
            className="btn-primary"
            onClick={handleNext}
            disabled={isGenerating}
          >
            Next →
          </button>
        ) : (
          <button 
            className="btn-primary generate-btn"
            onClick={handleGenerate}
            disabled={isGenerating || !paAgent}
          >
            {isGenerating ? (
              <>
                <div className="spinner" />
                Generating Plan...
              </>
            ) : (
              <>
                ✨ Generate Plan
              </>
            )}
          </button>
        )}
      </div>
    </div>
  );
};

// Step 1: Describe your goals
const DescribeStep: React.FC<{
  formData: PlanGenerationRequest;
  errors: Record<string, string>;
  onUpdate: (field: keyof PlanGenerationRequest, value: any) => void;
}> = ({ formData, errors, onUpdate }) => {
  const quickTemplates = [
    {
      title: "Build a Web Application",
      description: "Create a modern web application with user authentication, database integration, and responsive design. Include frontend, backend, and deployment phases."
    },
    {
      title: "Data Analysis Project", 
      description: "Analyze a dataset to extract insights and create visualizations. Include data cleaning, statistical analysis, and report generation."
    },
    {
      title: "Code Refactoring",
      description: "Refactor existing codebase to improve maintainability, performance, and code quality. Include testing and documentation updates."
    },
    {
      title: "API Integration",
      description: "Integrate with external APIs to add new functionality. Include authentication, error handling, and data transformation."
    }
  ];
  
  return (
    <div className="describe-step">
      <div className="step-section">
        <h2>What do you want to accomplish?</h2>
        
        {/* Title Input */}
        <div className="form-group">
          <label htmlFor="title">Project Title *</label>
          <input
            id="title"
            type="text"
            value={formData.title}
            onChange={(e) => onUpdate('title', e.target.value)}
            placeholder="Enter a clear, descriptive title for your project"
            className={errors.title ? 'error' : ''}
          />
          {errors.title && <span className="error-message">{errors.title}</span>}
        </div>
        
        {/* Description Input */}
        <div className="form-group">
          <label htmlFor="description">Detailed Description *</label>
          <textarea
            id="description"
            value={formData.description}
            onChange={(e) => onUpdate('description', e.target.value)}
            placeholder="Describe your project in detail. What are your goals? What should the end result look like? Include any specific requirements or constraints."
            rows={6}
            className={errors.description ? 'error' : ''}
          />
          {errors.description && <span className="error-message">{errors.description}</span>}
          <div className="char-count">
            {formData.description.length} characters
          </div>
        </div>
        
        {/* Context Input */}
        <div className="form-group">
          <label htmlFor="context">Additional Context (Optional)</label>
          <textarea
            id="context"
            value={formData.context || ''}
            onChange={(e) => onUpdate('context', e.target.value)}
            placeholder="Any additional context, constraints, or specific requirements that should be considered during planning."
            rows={3}
          />
        </div>
      </div>
      
      {/* Quick Templates */}
      <div className="step-section">
        <h3>🚀 Quick Start Templates</h3>
        <div className="template-grid">
          {quickTemplates.map((template, index) => (
            <div 
              key={index}
              className="template-card"
              onClick={() => {
                onUpdate('title', template.title);
                onUpdate('description', template.description);
              }}
            >
              <h4>{template.title}</h4>
              <p>{template.description.substring(0, 100)}...</p>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
};

// Step 2: Configure planning options
const ConfigureStep: React.FC<{
  formData: PlanGenerationRequest;
  errors: Record<string, string>;
  onUpdate: (field: keyof PlanGenerationRequest, value: any) => void;
}> = ({ formData, errors, onUpdate }) => {
  return (
    <div className="configure-step">
      <div className="step-section">
        <h2>Configure Planning Options</h2>
        
        {/* Priority Selection */}
        <div className="form-group">
          <label>Project Priority</label>
          <div className="priority-options">
            {Object.values(TaskPriority).map(priority => (
              <button
                key={priority}
                className={`priority-btn ${formData.priority === priority ? 'selected' : ''}`}
                onClick={() => onUpdate('priority', priority)}
              >
                <div className={`priority-indicator ${priority}`} />
                {priority.charAt(0).toUpperCase() + priority.slice(1)}
              </button>
            ))}
          </div>
        </div>
        
        {/* Deadline */}
        <div className="form-group">
          <label htmlFor="deadline">Target Deadline (Optional)</label>
          <input
            id="deadline"
            type="datetime-local"
            value={formData.deadline || ''}
            onChange={(e) => onUpdate('deadline', e.target.value)}
          />
        </div>
        
        {/* Planning Strategy */}
        <div className="form-group">
          <label>Planning Strategy</label>
          <div className="strategy-options">
            <div 
              className={`strategy-card ${formData.preferred_strategy === PlanningStrategy.Sequential ? 'selected' : ''}`}
              onClick={() => onUpdate('preferred_strategy', PlanningStrategy.Sequential)}
            >
              <h4>📋 Sequential</h4>
              <p>Tasks executed one after another in logical order</p>
            </div>
            
            <div 
              className={`strategy-card ${formData.preferred_strategy === PlanningStrategy.Parallel ? 'selected' : ''}`}
              onClick={() => onUpdate('preferred_strategy', PlanningStrategy.Parallel)}
            >
              <h4>⚡ Parallel</h4>
              <p>Multiple tasks executed simultaneously for speed</p>
            </div>
            
            <div 
              className={`strategy-card ${formData.preferred_strategy === PlanningStrategy.Hybrid ? 'selected' : ''}`}
              onClick={() => onUpdate('preferred_strategy', PlanningStrategy.Hybrid)}
            >
              <h4>🔄 Hybrid</h4>
              <p>Combination of sequential and parallel execution</p>
            </div>
            
            <div 
              className={`strategy-card ${formData.preferred_strategy === PlanningStrategy.Agile ? 'selected' : ''}`}
              onClick={() => onUpdate('preferred_strategy', PlanningStrategy.Agile)}
            >
              <h4>🏃 Agile</h4>
              <p>Iterative development with user stories and sprints</p>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};

// Step 3: Review and generate
const ReviewStep: React.FC<{
  formData: PlanGenerationRequest;
  paAgent: PAAgent | null;
}> = ({ formData, paAgent }) => {
  return (
    <div className="review-step">
      <div className="step-section">
        <h2>Review Your Plan Request</h2>
        
        <div className="review-card">
          <div className="review-item">
            <h3>📝 Project Title</h3>
            <p>{formData.title}</p>
          </div>
          
          <div className="review-item">
            <h3>📄 Description</h3>
            <p>{formData.description}</p>
          </div>
          
          {formData.context && (
            <div className="review-item">
              <h3>🔍 Additional Context</h3>
              <p>{formData.context}</p>
            </div>
          )}
          
          <div className="review-item">
            <h3>⚡ Priority</h3>
            <div className="priority-display">
              <div className={`priority-indicator ${formData.priority}`} />
              {formData.priority ? formData.priority.charAt(0).toUpperCase() + formData.priority.slice(1) : 'Not set'}
            </div>
          </div>
          
          {formData.deadline && (
            <div className="review-item">
              <h3>📅 Deadline</h3>
              <p>{new Date(formData.deadline).toLocaleString()}</p>
            </div>
          )}
          
          <div className="review-item">
            <h3>🎯 Strategy</h3>
            <p>{formData.preferred_strategy ? formData.preferred_strategy.charAt(0).toUpperCase() + formData.preferred_strategy.slice(1) : 'Not set'}</p>
          </div>
        </div>
        
        {/* PA Agent Status */}
        <div className="pa-status-card">
          <h3>🤖 PA Agent Status</h3>
          {paAgent ? (
            <div className="agent-info">
              <div className="status-indicator active" />
              <div>
                <p><strong>Agent ID:</strong> {paAgent.agent_id}</p>
                <p><strong>Model:</strong> {paAgent.model_provider}/{paAgent.model_name}</p>
                <p><strong>Status:</strong> {paAgent.status}</p>
              </div>
            </div>
          ) : (
            <div className="agent-info">
              <div className="status-indicator inactive" />
              <p>PA Agent not available</p>
            </div>
          )}
        </div>
      </div>
    </div>
  );
};
