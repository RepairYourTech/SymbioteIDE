/**
 * Voice Input Handler
 * 
 * Handles voice input and speech-to-text transcription
 */

import * as vscode from 'vscode';
import { VoiceInput } from '../../../types/chat-api';

export interface VoiceInputOptions {
  format?: 'webm' | 'mp3' | 'wav';
  sampleRate?: number;
  maxDuration?: number; // in seconds
}

export class VoiceInputHandler {
  private mediaRecorder: MediaRecorder | null = null;
  private audioChunks: Blob[] = [];
  private startTime: number = 0;
  
  constructor(private options: VoiceInputOptions = {}) {
    this.options = {
      format: 'webm',
      sampleRate: 16000,
      maxDuration: 120, // 2 minutes
      ...options
    };
  }
  
  /**
   * Start recording audio from microphone
   */
  async startRecording(): Promise<void> {
    try {
      const stream = await navigator.mediaDevices.getUserMedia({ 
        audio: {
          sampleRate: this.options.sampleRate,
          echoCancellation: true,
          noiseSuppression: true,
          autoGainControl: true
        } 
      });
      
      this.audioChunks = [];
      this.startTime = Date.now();
      
      // Create media recorder
      const mimeType = `audio/${this.options.format}`;
      this.mediaRecorder = new MediaRecorder(stream, {
        mimeType: MediaRecorder.isTypeSupported(mimeType) ? mimeType : 'audio/webm'
      });
      
      // Handle data available
      this.mediaRecorder.ondataavailable = (event) => {
        if (event.data.size > 0) {
          this.audioChunks.push(event.data);
        }
      };
      
      // Start recording
      this.mediaRecorder.start(100); // Collect data every 100ms
      
      // Auto-stop after max duration
      if (this.options.maxDuration) {
        setTimeout(() => {
          if (this.mediaRecorder?.state === 'recording') {
            this.stopRecording();
          }
        }, this.options.maxDuration * 1000);
      }
      
    } catch (error) {
      throw new Error(`Failed to start recording: ${error.message}`);
    }
  }
  
  /**
   * Stop recording and return voice input
   */
  async stopRecording(): Promise<VoiceInput> {
    if (!this.mediaRecorder || this.mediaRecorder.state !== 'recording') {
      throw new Error('No active recording');
    }
    
    return new Promise((resolve, reject) => {
      this.mediaRecorder!.onstop = async () => {
        try {
          // Create blob from chunks
          const audioBlob = new Blob(this.audioChunks, { 
            type: `audio/${this.options.format}` 
          });
          
          // Convert to base64
          const reader = new FileReader();
          reader.onloadend = () => {
            const base64Audio = reader.result as string;
            
            resolve({
              audio: base64Audio,
              format: this.options.format!,
              duration: (Date.now() - this.startTime) / 1000,
              sampleRate: this.options.sampleRate
            });
          };
          
          reader.onerror = () => {
            reject(new Error('Failed to convert audio to base64'));
          };
          
          reader.readAsDataURL(audioBlob);
          
          // Stop all tracks
          this.mediaRecorder!.stream.getTracks().forEach(track => track.stop());
          
        } catch (error) {
          reject(error);
        }
      };
      
      this.mediaRecorder!.stop();
    });
  }
  
  /**
   * Cancel recording
   */
  cancelRecording(): void {
    if (this.mediaRecorder && this.mediaRecorder.state === 'recording') {
      this.mediaRecorder.stream.getTracks().forEach(track => track.stop());
      this.mediaRecorder.stop();
      this.audioChunks = [];
    }
  }
  
  /**
   * Check if recording is active
   */
  isRecording(): boolean {
    return this.mediaRecorder?.state === 'recording' || false;
  }
  
  /**
   * Get recording duration in seconds
   */
  getRecordingDuration(): number {
    if (!this.isRecording()) return 0;
    return (Date.now() - this.startTime) / 1000;
  }
}

/**
 * Speech-to-text transcription service
 */
export class SpeechTranscriptionService {
  constructor(private apiKey?: string) {}
  
  /**
   * Transcribe audio to text using AI service
   */
  async transcribe(voiceInput: VoiceInput): Promise<string> {
    // This would integrate with a speech-to-text service like:
    // - OpenAI Whisper API
    // - Google Speech-to-Text
    // - Azure Speech Services
    // - Local Whisper model
    
    // For now, return a placeholder
    return `[Voice input transcription - ${voiceInput.duration}s of audio]`;
  }
  
  /**
   * Detect language from audio
   */
  async detectLanguage(voiceInput: VoiceInput): Promise<string> {
    // Language detection logic
    return 'en-US';
  }
}