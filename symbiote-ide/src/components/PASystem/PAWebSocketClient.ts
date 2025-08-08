// PA WebSocket Client - Frontend WebSocket client for real-time PA system communication
import { PASystemEvent, PAWebSocketMessage } from './types';

export class PAWebSocketClient {
  private ws: WebSocket | null = null;
  private reconnectAttempts = 0;
  private maxReconnectAttempts = 5;
  private reconnectDelay = 1000;
  private isConnecting = false;
  
  public onEvent: ((event: PASystemEvent) => void) | null = null;
  public onConnectionChange: ((connected: boolean) => void) | null = null;
  public onError: ((error: string) => void) | null = null;
  
  constructor(private url: string) {}
  
  async connect(): Promise<void> {
    if (this.isConnecting || (this.ws && this.ws.readyState === WebSocket.OPEN)) {
      return;
    }
    
    this.isConnecting = true;
    
    try {
      this.ws = new WebSocket(this.url);
      
      this.ws.onopen = () => {
        console.log('PA WebSocket connected');
        this.isConnecting = false;
        this.reconnectAttempts = 0;
        this.onConnectionChange?.(true);
        
        // Send connection message
        this.send({
          type: 'Connected',
          timestamp: new Date().toISOString(),
        });
      };
      
      this.ws.onmessage = (event) => {
        try {
          const data = JSON.parse(event.data);
          this.handleMessage(data);
        } catch (error) {
          console.error('Failed to parse WebSocket message:', error);
        }
      };
      
      this.ws.onclose = () => {
        console.log('PA WebSocket disconnected');
        this.isConnecting = false;
        this.onConnectionChange?.(false);
        this.attemptReconnect();
      };
      
      this.ws.onerror = (error) => {
        console.error('PA WebSocket error:', error);
        this.isConnecting = false;
        this.onError?.('WebSocket connection error');
      };
      
    } catch (error) {
      this.isConnecting = false;
      this.onError?.(`Failed to connect: ${error}`);
      this.attemptReconnect();
    }
  }
  
  disconnect(): void {
    if (this.ws) {
      this.ws.close();
      this.ws = null;
    }
    this.reconnectAttempts = this.maxReconnectAttempts; // Prevent reconnection
  }
  
  send(message: PAWebSocketMessage): void {
    if (this.ws && this.ws.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify(message));
    } else {
      console.warn('WebSocket not connected, message not sent:', message);
    }
  }
  
  private handleMessage(data: any): void {
    // Handle different message types
    if (data.type && this.isSystemEvent(data.type)) {
      const event: PASystemEvent = {
        type: data.type,
        timestamp: data.timestamp || new Date().toISOString(),
        data: data.data,
        task_id: data.task_id,
        agent_id: data.agent_id,
        plan_id: data.plan_id,
        status: data.status,
      };
      
      this.onEvent?.(event);
    }
  }
  
  private isSystemEvent(type: string): boolean {
    const systemEvents = [
      'TaskCreated',
      'TaskUpdated', 
      'TaskCompleted',
      'PlanGenerated',
      'AgentAssigned',
      'AgentStatusChanged'
    ];
    return systemEvents.includes(type);
  }
  
  private attemptReconnect(): void {
    if (this.reconnectAttempts >= this.maxReconnectAttempts) {
      console.log('Max reconnection attempts reached');
      return;
    }
    
    this.reconnectAttempts++;
    const delay = this.reconnectDelay * Math.pow(2, this.reconnectAttempts - 1);
    
    console.log(`Attempting to reconnect in ${delay}ms (attempt ${this.reconnectAttempts})`);
    
    setTimeout(() => {
      this.connect();
    }, delay);
  }
  
  // PA System specific methods
  createTask(title: string, description: string): void {
    this.send({
      type: 'CreateTask',
      data: { title, description },
      timestamp: new Date().toISOString(),
    });
  }
  
  updateTask(taskId: string, updates: any): void {
    this.send({
      type: 'UpdateTask',
      task_id: taskId,
      data: updates,
      timestamp: new Date().toISOString(),
    });
  }
  
  generatePlan(request: any): void {
    this.send({
      type: 'GeneratePlan',
      data: request,
      timestamp: new Date().toISOString(),
    });
  }
  
  assignAgent(taskId: string, agentId: string): void {
    this.send({
      type: 'AssignAgent',
      task_id: taskId,
      agent_id: agentId,
      timestamp: new Date().toISOString(),
    });
  }
  
  isConnected(): boolean {
    return this.ws !== null && this.ws.readyState === WebSocket.OPEN;
  }
}
