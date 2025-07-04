"""
Mem0 Memory Service - FastAPI wrapper for Mem0
"""

import os
import logging
from typing import List, Dict, Any, Optional
from fastapi import FastAPI, HTTPException
from fastapi.middleware.cors import CORSMiddleware
from pydantic import BaseModel
from mem0 import Memory
import redis
from qdrant_client import QdrantClient

# Configure logging
logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)

# Initialize FastAPI app
app = FastAPI(title="Symbiote Mem0 Service", version="1.0.0")

# Add CORS middleware
app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

# Configuration from environment
QDRANT_URL = os.getenv("QDRANT_URL", "http://qdrant:6333")
REDIS_URL = os.getenv("REDIS_URL", "redis://redis:6379")
REDIS_PASSWORD = os.getenv("REDIS_PASSWORD", "symbiote123")

# Initialize Mem0
config = {
    "vector_store": {
        "provider": "qdrant",
        "config": {
            "url": QDRANT_URL,
            "collection_name": "symbiote_memories"
        }
    },
    "llm": {
        "provider": "openai",  # Will be overridden by orchestrator
        "config": {
            "model": "gpt-3.5-turbo",
            "temperature": 0
        }
    },
    "embedder": {
        "provider": "openai",  # Will be overridden by orchestrator
        "config": {
            "model": "text-embedding-ada-002"
        }
    }
}

memory = Memory.from_config(config)

# Redis client for caching
redis_client = redis.from_url(REDIS_URL, password=REDIS_PASSWORD, decode_responses=True)

# Request/Response models
class AddMemoryRequest(BaseModel):
    messages: List[Dict[str, str]]
    user_id: str
    metadata: Optional[Dict[str, Any]] = None

class SearchMemoryRequest(BaseModel):
    query: str
    user_id: str
    limit: int = 5

class UpdateMemoryRequest(BaseModel):
    memory_id: str
    data: str

class MemoryResponse(BaseModel):
    id: str
    memory: str
    metadata: Dict[str, Any]

# Health check endpoint
@app.get("/health")
async def health_check():
    try:
        # Check Redis
        redis_client.ping()
        
        # Check Qdrant
        qdrant = QdrantClient(url=QDRANT_URL)
        qdrant.get_collections()
        
        return {"status": "healthy", "service": "mem0"}
    except Exception as e:
        logger.error(f"Health check failed: {e}")
        raise HTTPException(status_code=503, detail=str(e))

# Add memory
@app.post("/api/memories")
async def add_memory(request: AddMemoryRequest):
    try:
        result = memory.add(
            messages=request.messages,
            user_id=request.user_id,
            metadata=request.metadata
        )
        
        # Cache in Redis
        cache_key = f"memory:{request.user_id}:latest"
        redis_client.setex(cache_key, 3600, str(result))
        
        return {"success": True, "memory_id": result}
    except Exception as e:
        logger.error(f"Failed to add memory: {e}")
        raise HTTPException(status_code=500, detail=str(e))

# Search memories
@app.post("/api/memories/search")
async def search_memories(request: SearchMemoryRequest):
    try:
        # Check cache first
        cache_key = f"search:{request.user_id}:{request.query}"
        cached = redis_client.get(cache_key)
        
        if cached:
            import json
            return json.loads(cached)
        
        # Search in Mem0
        results = memory.search(
            query=request.query,
            user_id=request.user_id,
            limit=request.limit
        )
        
        # Format results
        memories = []
        for result in results:
            memories.append({
                "id": result.get("id"),
                "memory": result.get("memory"),
                "metadata": result.get("metadata", {}),
                "score": result.get("score", 0)
            })
        
        # Cache results
        redis_client.setex(cache_key, 1800, str(memories))
        
        return {"memories": memories}
    except Exception as e:
        logger.error(f"Failed to search memories: {e}")
        raise HTTPException(status_code=500, detail=str(e))

# Get all memories for user
@app.get("/api/memories/{user_id}")
async def get_user_memories(user_id: str):
    try:
        results = memory.get_all(user_id=user_id)
        
        memories = []
        for result in results:
            memories.append({
                "id": result.get("id"),
                "memory": result.get("memory"),
                "metadata": result.get("metadata", {}),
                "created_at": result.get("created_at")
            })
        
        return {"user_id": user_id, "memories": memories}
    except Exception as e:
        logger.error(f"Failed to get user memories: {e}")
        raise HTTPException(status_code=500, detail=str(e))

# Get specific memory
@app.get("/api/memories/{user_id}/{memory_id}")
async def get_memory(user_id: str, memory_id: str):
    try:
        result = memory.get(memory_id=memory_id)
        
        if not result:
            raise HTTPException(status_code=404, detail="Memory not found")
        
        return {
            "id": result.get("id"),
            "memory": result.get("memory"),
            "metadata": result.get("metadata", {}),
            "user_id": user_id
        }
    except HTTPException:
        raise
    except Exception as e:
        logger.error(f"Failed to get memory: {e}")
        raise HTTPException(status_code=500, detail=str(e))

# Update memory
@app.put("/api/memories/{memory_id}")
async def update_memory(memory_id: str, request: UpdateMemoryRequest):
    try:
        memory.update(memory_id=memory_id, data=request.data)
        
        # Invalidate cache
        redis_client.delete(f"memory:*:{memory_id}")
        
        return {"success": True, "memory_id": memory_id}
    except Exception as e:
        logger.error(f"Failed to update memory: {e}")
        raise HTTPException(status_code=500, detail=str(e))

# Delete memory
@app.delete("/api/memories/{memory_id}")
async def delete_memory(memory_id: str):
    try:
        memory.delete(memory_id=memory_id)
        
        # Invalidate cache
        redis_client.delete(f"memory:*:{memory_id}")
        
        return {"success": True, "memory_id": memory_id}
    except Exception as e:
        logger.error(f"Failed to delete memory: {e}")
        raise HTTPException(status_code=500, detail=str(e))

# Delete all memories for user
@app.delete("/api/memories/user/{user_id}")
async def delete_user_memories(user_id: str):
    try:
        memory.delete_all(user_id=user_id)
        
        # Clear user cache
        for key in redis_client.scan_iter(f"*:{user_id}:*"):
            redis_client.delete(key)
        
        return {"success": True, "user_id": user_id}
    except Exception as e:
        logger.error(f"Failed to delete user memories: {e}")
        raise HTTPException(status_code=500, detail=str(e))

# Get memory history for user
@app.get("/api/memories/{user_id}/history")
async def get_memory_history(user_id: str):
    try:
        history = memory.history(user_id=user_id)
        
        return {
            "user_id": user_id,
            "history": history
        }
    except Exception as e:
        logger.error(f"Failed to get memory history: {e}")
        raise HTTPException(status_code=500, detail=str(e))

# Configure LLM provider
@app.post("/api/config/llm")
async def configure_llm(config: Dict[str, Any]):
    try:
        # Update Mem0 LLM configuration
        memory.llm.update_config(config)
        
        return {"success": True, "config": config}
    except Exception as e:
        logger.error(f"Failed to configure LLM: {e}")
        raise HTTPException(status_code=500, detail=str(e))

# Configure embedder
@app.post("/api/config/embedder")
async def configure_embedder(config: Dict[str, Any]):
    try:
        # Update Mem0 embedder configuration
        memory.embedder.update_config(config)
        
        return {"success": True, "config": config}
    except Exception as e:
        logger.error(f"Failed to configure embedder: {e}")
        raise HTTPException(status_code=500, detail=str(e))

# Get service stats
@app.get("/api/stats")
async def get_stats():
    try:
        # Get collection stats from Qdrant
        qdrant = QdrantClient(url=QDRANT_URL)
        collection_info = qdrant.get_collection("symbiote_memories")
        
        # Get Redis stats
        redis_info = redis_client.info()
        
        return {
            "memories": {
                "total": collection_info.vectors_count,
                "indexed": collection_info.indexed_vectors_count
            },
            "cache": {
                "keys": redis_client.dbsize(),
                "memory_usage": redis_info.get("used_memory_human", "0")
            }
        }
    except Exception as e:
        logger.error(f"Failed to get stats: {e}")
        raise HTTPException(status_code=500, detail=str(e))

if __name__ == "__main__":
    import uvicorn
    uvicorn.run(app, host="0.0.0.0", port=8000)