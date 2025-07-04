/**
 * Room Manager
 * 
 * Manages WebSocket rooms for organized communication
 */

import { Server as SocketServer } from 'socket.io';
import { Logger } from '../utils/logger';
import { Room, RoomType, IRoomManager } from './types';

export class RoomManager implements IRoomManager {
  private io: SocketServer;
  private rooms = new Map<string, Room>();
  private logger = new Logger('RoomManager');
  
  constructor(io: SocketServer) {
    this.io = io;
  }
  
  /**
   * Create a new room
   */
  async createRoom(room: Room): Promise<void> {
    if (this.rooms.has(room.id)) {
      throw new Error(`Room ${room.id} already exists`);
    }
    
    this.rooms.set(room.id, room);
    this.logger.info('Room created', { room: room.id, type: room.type });
  }
  
  /**
   * Delete a room
   */
  async deleteRoom(roomId: string): Promise<void> {
    const room = this.rooms.get(roomId);
    if (!room) {
      throw new Error(`Room ${roomId} not found`);
    }
    
    // Remove all members
    const sockets = await this.io.in(roomId).fetchSockets();
    for (const socket of sockets) {
      socket.leave(roomId);
    }
    
    this.rooms.delete(roomId);
    this.logger.info('Room deleted', { room: roomId });
  }
  
  /**
   * Join a room
   */
  async joinRoom(socketId: string, roomId: string): Promise<void> {
    const room = this.rooms.get(roomId);
    if (!room) {
      // Auto-create room if it doesn't exist
      await this.createRoom({
        id: roomId,
        type: this.inferRoomType(roomId),
        name: roomId,
        members: [],
        created: new Date()
      });
    }
    
    // Get socket
    const sockets = await this.io.fetchSockets();
    const socket = sockets.find(s => s.id === socketId);
    
    if (!socket) {
      throw new Error(`Socket ${socketId} not found`);
    }
    
    // Join room
    socket.join(roomId);
    
    // Update room members
    const updatedRoom = this.rooms.get(roomId)!;
    if (!updatedRoom.members.includes(socketId)) {
      updatedRoom.members.push(socketId);
    }
    
    this.logger.debug('Socket joined room', { socket: socketId, room: roomId });
  }
  
  /**
   * Leave a room
   */
  async leaveRoom(socketId: string, roomId: string): Promise<void> {
    // Get socket
    const sockets = await this.io.fetchSockets();
    const socket = sockets.find(s => s.id === socketId);
    
    if (socket) {
      socket.leave(roomId);
    }
    
    // Update room members
    const room = this.rooms.get(roomId);
    if (room) {
      room.members = room.members.filter(id => id !== socketId);
      
      // Delete room if empty and not a persistent type
      if (room.members.length === 0 && 
          room.type !== RoomType.Project && 
          room.type !== RoomType.Team) {
        await this.deleteRoom(roomId);
      }
    }
    
    this.logger.debug('Socket left room', { socket: socketId, room: roomId });
  }
  
  /**
   * Get room members
   */
  async getRoomMembers(roomId: string): Promise<string[]> {
    const room = this.rooms.get(roomId);
    return room ? [...room.members] : [];
  }
  
  /**
   * Get user rooms
   */
  async getUserRooms(socketId: string): Promise<Room[]> {
    const userRooms: Room[] = [];
    
    for (const room of this.rooms.values()) {
      if (room.members.includes(socketId)) {
        userRooms.push({ ...room });
      }
    }
    
    return userRooms;
  }
  
  /**
   * Broadcast to room
   */
  async broadcastToRoom(
    roomId: string, 
    event: string, 
    data: any, 
    exclude?: string[]
  ): Promise<void> {
    let emitter = this.io.to(roomId);
    
    if (exclude && exclude.length > 0) {
      exclude.forEach(id => {
        emitter = emitter.except(id);
      });
    }
    
    emitter.emit(event, data);
  }
  
  /**
   * Get room info
   */
  getRoom(roomId: string): Room | undefined {
    return this.rooms.get(roomId);
  }
  
  /**
   * Get all rooms
   */
  getAllRooms(): Room[] {
    return Array.from(this.rooms.values());
  }
  
  /**
   * Get rooms by type
   */
  getRoomsByType(type: RoomType): Room[] {
    return Array.from(this.rooms.values()).filter(room => room.type === type);
  }
  
  /**
   * Clean up empty rooms
   */
  async cleanupEmptyRooms(): Promise<number> {
    let cleaned = 0;
    
    for (const [roomId, room] of this.rooms) {
      // Check actual socket membership
      const sockets = await this.io.in(roomId).fetchSockets();
      
      if (sockets.length === 0 && 
          room.type !== RoomType.Project && 
          room.type !== RoomType.Team) {
        await this.deleteRoom(roomId);
        cleaned++;
      } else {
        // Update member list
        room.members = sockets.map(s => s.id);
      }
    }
    
    if (cleaned > 0) {
      this.logger.info(`Cleaned up ${cleaned} empty rooms`);
    }
    
    return cleaned;
  }
  
  /**
   * Infer room type from ID
   */
  private inferRoomType(roomId: string): RoomType {
    if (roomId.startsWith('project:')) return RoomType.Project;
    if (roomId.startsWith('team:')) return RoomType.Team;
    if (roomId.startsWith('agent:')) return RoomType.Agent;
    if (roomId.startsWith('file:')) return RoomType.File;
    if (roomId.startsWith('task:')) return RoomType.Task;
    return RoomType.Private;
  }
}