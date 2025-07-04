/**
 * Embedding Service
 * 
 * Generates embeddings for code using various providers
 */

import OpenAI from 'openai';
import { Anthropic } from '@anthropic-ai/sdk';
import { GoogleGenerativeAI } from '@google/generative-ai';
import { Logger } from '../../utils/logger';
import { EmbeddingProvider, EmbeddingConfig, CodeEmbeddingMetadata } from './types';

export class EmbeddingService {
  private config: EmbeddingConfig;
  private openai?: OpenAI;
  private anthropic?: Anthropic;
  private google?: GoogleGenerativeAI;
  private logger = new Logger('EmbeddingService');
  private cache = new Map<string, number[]>();
  
  constructor(config: EmbeddingConfig) {
    this.config = config;
    this.initializeProvider();
  }
  
  /**
   * Initialize the embedding provider
   */
  private initializeProvider(): void {
    switch (this.config.provider) {
      case EmbeddingProvider.OpenAI:
        this.openai = new OpenAI({
          apiKey: this.config.options?.apiKey || process.env.OPENAI_API_KEY
        });
        break;
        
      case EmbeddingProvider.Anthropic:
        this.anthropic = new Anthropic({
          apiKey: this.config.options?.apiKey || process.env.ANTHROPIC_API_KEY
        });
        break;
        
      case EmbeddingProvider.Google:
        this.google = new GoogleGenerativeAI(
          this.config.options?.apiKey || process.env.GOOGLE_API_KEY || ''
        );
        break;
        
      case EmbeddingProvider.Local:
        // Initialize local model (e.g., using Sentence Transformers)
        this.initializeLocalModel();
        break;
        
      case EmbeddingProvider.Custom:
        // Custom provider initialization
        break;
    }
  }
  
  /**
   * Generate embedding for text
   */
  async embed(text: string): Promise<number[]> {
    // Check cache
    const cacheKey = this.getCacheKey(text);
    if (this.cache.has(cacheKey)) {
      return this.cache.get(cacheKey)!;
    }
    
    let embedding: number[];
    
    switch (this.config.provider) {
      case EmbeddingProvider.OpenAI:
        embedding = await this.embedWithOpenAI(text);
        break;
        
      case EmbeddingProvider.Anthropic:
        embedding = await this.embedWithAnthropic(text);
        break;
        
      case EmbeddingProvider.Google:
        embedding = await this.embedWithGoogle(text);
        break;
        
      case EmbeddingProvider.Local:
        embedding = await this.embedWithLocal(text);
        break;
        
      default:
        throw new Error(`Unsupported embedding provider: ${this.config.provider}`);
    }
    
    // Cache the result
    this.cache.set(cacheKey, embedding);
    
    return embedding;
  }
  
  /**
   * Batch embed multiple texts
   */
  async batchEmbed(texts: string[]): Promise<number[][]> {
    const results: number[][] = [];
    
    // Check cache and separate cached/uncached
    const uncachedTexts: string[] = [];
    const uncachedIndices: number[] = [];
    
    for (let i = 0; i < texts.length; i++) {
      const cacheKey = this.getCacheKey(texts[i]);
      if (this.cache.has(cacheKey)) {
        results[i] = this.cache.get(cacheKey)!;
      } else {
        uncachedTexts.push(texts[i]);
        uncachedIndices.push(i);
      }
    }
    
    if (uncachedTexts.length === 0) {
      return results;
    }
    
    // Process uncached texts in batches
    const batchSize = this.config.batchSize || 100;
    for (let i = 0; i < uncachedTexts.length; i += batchSize) {
      const batch = uncachedTexts.slice(i, i + batchSize);
      const embeddings = await this.processBatch(batch);
      
      // Store results and update cache
      for (let j = 0; j < embeddings.length; j++) {
        const originalIndex = uncachedIndices[i + j];
        results[originalIndex] = embeddings[j];
        this.cache.set(this.getCacheKey(batch[j]), embeddings[j]);
      }
    }
    
    return results;
  }
  
  /**
   * Generate embedding for code with context
   */
  async embedCode(code: string, language: string): Promise<number[]> {
    // Enhance code with language context
    const enhancedText = this.enhanceCodeForEmbedding(code, language);
    return this.embed(enhancedText);
  }
  
  /**
   * Generate embedding with full context
   */
  async embedWithContext(code: string, context: CodeEmbeddingMetadata): Promise<number[]> {
    // Build comprehensive text representation
    const parts: string[] = [];
    
    // Add language and type
    parts.push(`Language: ${context.language}`);
    parts.push(`Type: ${context.type}`);
    
    // Add name and signature
    if (context.name) parts.push(`Name: ${context.name}`);
    if (context.signature) parts.push(`Signature: ${context.signature}`);
    
    // Add documentation
    if (context.docstring) parts.push(`Documentation: ${context.docstring}`);
    if (context.description) parts.push(`Description: ${context.description}`);
    
    // Add the code itself
    parts.push('Code:');
    parts.push(code);
    
    // Add imports/exports for context
    if (context.imports?.length) {
      parts.push(`Imports: ${context.imports.join(', ')}`);
    }
    if (context.exports?.length) {
      parts.push(`Exports: ${context.exports.join(', ')}`);
    }
    
    const enhancedText = parts.join('\n');
    return this.embed(enhancedText);
  }
  
  /**
   * Embed with OpenAI
   */
  private async embedWithOpenAI(text: string): Promise<number[]> {
    if (!this.openai) {
      throw new Error('OpenAI client not initialized');
    }
    
    try {
      const response = await this.openai.embeddings.create({
        model: this.config.model || 'text-embedding-3-large',
        input: text.slice(0, this.config.maxTokens || 8192),
        dimensions: this.config.dimensions
      });
      
      return response.data[0].embedding;
    } catch (error) {
      this.logger.error('OpenAI embedding failed', error);
      throw error;
    }
  }
  
  /**
   * Embed with Anthropic (using their new embedding API when available)
   */
  private async embedWithAnthropic(text: string): Promise<number[]> {
    // Note: Anthropic doesn't have a direct embedding API yet
    // This is a placeholder for when they release one
    // For now, we could use their model to generate embeddings via prompting
    throw new Error('Anthropic embeddings not yet implemented');
  }
  
  /**
   * Embed with Google
   */
  private async embedWithGoogle(text: string): Promise<number[]> {
    if (!this.google) {
      throw new Error('Google client not initialized');
    }
    
    try {
      // Get the embedding model
      const model = this.google.getGenerativeModel({ 
        model: this.config.model || 'text-embedding-004'
      });
      
      // Generate embedding
      const result = await model.embedContent({
        content: { parts: [{ text: text.slice(0, this.config.maxTokens || 8192) }] },
        taskType: 'RETRIEVAL_DOCUMENT',
        outputDimensionality: this.config.dimensions
      });
      
      return result.embedding.values;
    } catch (error) {
      this.logger.error('Google embedding failed', error);
      throw error;
    }
  }
  
  /**
   * Embed with local model
   */
  private async embedWithLocal(text: string): Promise<number[]> {
    // This would integrate with a local model like Sentence Transformers
    // For now, returning a mock embedding
    this.logger.warn('Local embeddings not implemented, using mock data');
    return new Array(this.config.dimensions || 768).fill(0).map(() => Math.random());
  }
  
  /**
   * Initialize local model
   */
  private initializeLocalModel(): void {
    // Initialize local embedding model
    // This could use ONNX Runtime, TensorFlow.js, or other local inference
    this.logger.info('Local model initialization placeholder');
  }
  
  /**
   * Process a batch of texts
   */
  private async processBatch(texts: string[]): Promise<number[][]> {
    switch (this.config.provider) {
      case EmbeddingProvider.OpenAI:
        return this.batchEmbedWithOpenAI(texts);
        
      case EmbeddingProvider.Google:
        return this.batchEmbedWithGoogle(texts);
        
      case EmbeddingProvider.Local:
        // Process locally one by one for now
        return Promise.all(texts.map(text => this.embedWithLocal(text)));
        
      default:
        // Fallback to individual processing
        return Promise.all(texts.map(text => this.embed(text)));
    }
  }
  
  /**
   * Batch embed with OpenAI
   */
  private async batchEmbedWithOpenAI(texts: string[]): Promise<number[][]> {
    if (!this.openai) {
      throw new Error('OpenAI client not initialized');
    }
    
    try {
      const response = await this.openai.embeddings.create({
        model: this.config.model || 'text-embedding-3-large',
        input: texts.map(text => text.slice(0, this.config.maxTokens || 8192)),
        dimensions: this.config.dimensions
      });
      
      return response.data.map(item => item.embedding);
    } catch (error) {
      this.logger.error('OpenAI batch embedding failed', error);
      throw error;
    }
  }
  
  /**
   * Batch embed with Google
   */
  private async batchEmbedWithGoogle(texts: string[]): Promise<number[][]> {
    if (!this.google) {
      throw new Error('Google client not initialized');
    }
    
    try {
      const model = this.google.getGenerativeModel({ 
        model: this.config.model || 'text-embedding-004'
      });
      
      // Google's batch embedding API
      const batchResult = await model.batchEmbedContents({
        requests: texts.map(text => ({
          content: { parts: [{ text: text.slice(0, this.config.maxTokens || 8192) }] },
          taskType: 'RETRIEVAL_DOCUMENT',
          outputDimensionality: this.config.dimensions
        }))
      });
      
      return batchResult.embeddings.map(embedding => embedding.values);
    } catch (error) {
      this.logger.error('Google batch embedding failed', error);
      throw error;
    }
  }
  
  /**
   * Enhance code for better embedding
   */
  private enhanceCodeForEmbedding(code: string, language: string): string {
    const parts: string[] = [];
    
    // Add language context
    parts.push(`Programming Language: ${language}`);
    
    // Extract and add function/class names
    const names = this.extractNames(code, language);
    if (names.length > 0) {
      parts.push(`Identifiers: ${names.join(', ')}`);
    }
    
    // Extract and add comments
    const comments = this.extractComments(code, language);
    if (comments.length > 0) {
      parts.push(`Comments: ${comments.join(' ')}`);
    }
    
    // Add the code
    parts.push('Code:');
    parts.push(code);
    
    return parts.join('\n');
  }
  
  /**
   * Extract identifiers from code
   */
  private extractNames(code: string, language: string): string[] {
    const names: string[] = [];
    
    // Simple regex-based extraction (should use proper AST parsing)
    const patterns: Record<string, RegExp[]> = {
      javascript: [
        /(?:function|const|let|var|class)\s+(\w+)/g,
        /(\w+)\s*[:=]\s*(?:async\s+)?(?:function|\()/g
      ],
      typescript: [
        /(?:function|const|let|var|class|interface|type|enum)\s+(\w+)/g,
        /(\w+)\s*[:=]\s*(?:async\s+)?(?:function|\()/g
      ],
      python: [
        /(?:def|class)\s+(\w+)/g,
        /(\w+)\s*=/g
      ]
    };
    
    const langPatterns = patterns[language] || patterns.javascript;
    
    for (const pattern of langPatterns) {
      let match;
      while ((match = pattern.exec(code)) !== null) {
        if (match[1] && !names.includes(match[1])) {
          names.push(match[1]);
        }
      }
    }
    
    return names;
  }
  
  /**
   * Extract comments from code
   */
  private extractComments(code: string, language: string): string[] {
    const comments: string[] = [];
    
    // Extract single-line comments
    const singleLineRegex = /\/\/(.*)$/gm;
    let match;
    while ((match = singleLineRegex.exec(code)) !== null) {
      comments.push(match[1].trim());
    }
    
    // Extract multi-line comments
    const multiLineRegex = /\/\*[\s\S]*?\*\//g;
    while ((match = multiLineRegex.exec(code)) !== null) {
      const comment = match[0]
        .replace(/^\/\*\*?/, '')
        .replace(/\*\/$/, '')
        .replace(/^\s*\*\s?/gm, '')
        .trim();
      if (comment) comments.push(comment);
    }
    
    // Python-style comments
    if (language === 'python') {
      const pythonCommentRegex = /#(.*)$/gm;
      while ((match = pythonCommentRegex.exec(code)) !== null) {
        comments.push(match[1].trim());
      }
      
      // Docstrings
      const docstringRegex = /"""[\s\S]*?"""|'''[\s\S]*?'''/g;
      while ((match = docstringRegex.exec(code)) !== null) {
        const docstring = match[0].slice(3, -3).trim();
        if (docstring) comments.push(docstring);
      }
    }
    
    return comments;
  }
  
  /**
   * Get cache key for text
   */
  private getCacheKey(text: string): string {
    // Simple hash function for cache key
    let hash = 0;
    for (let i = 0; i < text.length; i++) {
      const char = text.charCodeAt(i);
      hash = ((hash << 5) - hash) + char;
      hash = hash & hash;
    }
    return `${this.config.provider}:${this.config.model}:${hash}`;
  }
  
  /**
   * Clear cache
   */
  clearCache(): void {
    this.cache.clear();
  }
  
  /**
   * Get model info
   */
  getModelInfo(): { provider: string; model: string; dimensions: number } {
    return {
      provider: this.config.provider,
      model: this.config.model,
      dimensions: this.config.dimensions
    };
  }
  
  /**
   * Switch model
   */
  async switchModel(config: EmbeddingConfig): Promise<void> {
    this.config = config;
    this.clearCache();
    this.initializeProvider();
  }
}