import React, { useState, useRef, useEffect } from 'react';
import { VSCodeButton } from '@vscode/webview-ui-toolkit/react';
import './VoiceInput.css';

interface VoiceInputProps {
  onVoiceInput: (transcript: string, audioData: string) => void;
  disabled?: boolean;
}

export const VoiceInput: React.FC<VoiceInputProps> = ({ onVoiceInput, disabled }) => {
  const [isRecording, setIsRecording] = useState(false);
  const [recordingTime, setRecordingTime] = useState(0);
  const [error, setError] = useState<string | null>(null);
  
  const mediaRecorderRef = useRef<MediaRecorder | null>(null);
  const audioChunksRef = useRef<Blob[]>([]);
  const timerRef = useRef<NodeJS.Timeout | null>(null);
  const startTimeRef = useRef<number>(0);

  useEffect(() => {
    return () => {
      // Cleanup on unmount
      if (timerRef.current) {
        clearInterval(timerRef.current);
      }
      stopRecording();
    };
  }, []);

  const startRecording = async () => {
    try {
      setError(null);
      
      // Request microphone permission
      const stream = await navigator.mediaDevices.getUserMedia({ 
        audio: {
          echoCancellation: true,
          noiseSuppression: true,
          autoGainControl: true
        } 
      });
      
      // Create media recorder
      const mediaRecorder = new MediaRecorder(stream, {
        mimeType: MediaRecorder.isTypeSupported('audio/webm') ? 'audio/webm' : 'audio/ogg'
      });
      
      mediaRecorderRef.current = mediaRecorder;
      audioChunksRef.current = [];
      
      // Handle data available
      mediaRecorder.ondataavailable = (event) => {
        if (event.data.size > 0) {
          audioChunksRef.current.push(event.data);
        }
      };
      
      // Handle recording stop
      mediaRecorder.onstop = async () => {
        // Create audio blob
        const audioBlob = new Blob(audioChunksRef.current, { type: 'audio/webm' });
        
        // Convert to base64
        const reader = new FileReader();
        reader.onloadend = () => {
          const base64Audio = reader.result as string;
          
          // Simulate transcription (in real app, send to transcription service)
          const duration = (Date.now() - startTimeRef.current) / 1000;
          const transcript = `[Voice message - ${duration.toFixed(1)}s]`;
          
          onVoiceInput(transcript, base64Audio);
        };
        reader.readAsDataURL(audioBlob);
        
        // Stop all tracks
        stream.getTracks().forEach(track => track.stop());
      };
      
      // Start recording
      mediaRecorder.start(100); // Collect data every 100ms
      setIsRecording(true);
      startTimeRef.current = Date.now();
      
      // Start timer
      timerRef.current = setInterval(() => {
        setRecordingTime(Math.floor((Date.now() - startTimeRef.current) / 1000));
      }, 100);
      
      // Auto-stop after 2 minutes
      setTimeout(() => {
        if (mediaRecorderRef.current?.state === 'recording') {
          stopRecording();
        }
      }, 120000);
      
    } catch (err) {
      setError('Microphone access denied');
      console.error('Failed to start recording:', err);
    }
  };

  const stopRecording = () => {
    if (mediaRecorderRef.current && mediaRecorderRef.current.state === 'recording') {
      mediaRecorderRef.current.stop();
      setIsRecording(false);
      setRecordingTime(0);
      
      if (timerRef.current) {
        clearInterval(timerRef.current);
        timerRef.current = null;
      }
    }
  };

  const cancelRecording = () => {
    if (mediaRecorderRef.current && mediaRecorderRef.current.state === 'recording') {
      // Stop without saving
      audioChunksRef.current = [];
      mediaRecorderRef.current.stream.getTracks().forEach(track => track.stop());
      mediaRecorderRef.current.stop();
      setIsRecording(false);
      setRecordingTime(0);
      
      if (timerRef.current) {
        clearInterval(timerRef.current);
        timerRef.current = null;
      }
    }
  };

  const formatTime = (seconds: number): string => {
    const mins = Math.floor(seconds / 60);
    const secs = seconds % 60;
    return `${mins}:${secs.toString().padStart(2, '0')}`;
  };

  if (!navigator.mediaDevices || !navigator.mediaDevices.getUserMedia) {
    return null; // Voice input not supported
  }

  return (
    <div className="voice-input-container">
      {isRecording ? (
        <div className="voice-recording">
          <div className="recording-indicator">
            <span className="recording-dot"></span>
            <span className="recording-time">{formatTime(recordingTime)}</span>
          </div>
          
          <div className="recording-waveform">
            {[...Array(5)].map((_, i) => (
              <div key={i} className="waveform-bar"></div>
            ))}
          </div>
          
          <div className="recording-actions">
            <VSCodeButton
              appearance="secondary"
              onClick={cancelRecording}
            >
              Cancel
            </VSCodeButton>
            <VSCodeButton
              appearance="primary"
              onClick={stopRecording}
            >
              <span className="codicon codicon-check"></span>
              Send
            </VSCodeButton>
          </div>
        </div>
      ) : (
        <VSCodeButton
          appearance="icon"
          aria-label="Start voice recording"
          onClick={startRecording}
          disabled={disabled}
          title="Hold to record voice message"
        >
          <span className="codicon codicon-mic"></span>
        </VSCodeButton>
      )}
      
      {error && (
        <div className="voice-error">
          {error}
        </div>
      )}
    </div>
  );
};