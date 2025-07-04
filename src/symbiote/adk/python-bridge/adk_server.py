#!/usr/bin/env python3
"""
Google ADK Python Bridge Server

This server provides a bridge between the TypeScript VS Code extension
and the Python-based Google AI ADK.
"""

import json
import asyncio
import logging
from typing import Dict, Any, Optional, List
from dataclasses import dataclass, asdict

import uvicorn
from fastapi import FastAPI, HTTPException
from fastapi.middleware.cors import CORSMiddleware
from pydantic import BaseModel

# Import the real Google ADK
from google.adk import Agent, Tool, LlmAgent, Configuration
from google.adk.orchestration import Orchestrator
from google.adk.memory import Memory
from google.adk.tools import ToolRegistry

# Configure logging
logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)

# FastAPI app
app = FastAPI(title="Google ADK Bridge Server")

# Add CORS middleware
app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

# Agent registry
agent_registry: Dict[str, Agent] = {}
tool_registry = ToolRegistry()


class AgentConfig(BaseModel):
    """Configuration for creating an agent"""
    id: str
    name: str
    type: str = "llm"
    description: Optional[str] = None
    system_prompt: Optional[str] = None
    model: Optional[str] = "gemini-1.5-pro"
    temperature: Optional[float] = 0.7
    tools: Optional[List[str]] = None
    memory_enabled: bool = False
    a2a_enabled: bool = False


class TaskRequest(BaseModel):
    """Request for executing a task"""
    agent_id: str
    task: str
    context: Optional[Dict[str, Any]] = None
    stream: bool = False


class ToolConfig(BaseModel):
    """Configuration for registering a tool"""
    name: str
    description: str
    parameters: Dict[str, Any]
    function_name: str


@app.get("/health")
async def health_check():
    """Health check endpoint"""
    return {"status": "healthy", "adk_version": "1.5.0"}


@app.post("/agents/create")
async def create_agent(config: AgentConfig):
    """Create a new agent"""
    try:
        # Create agent configuration
        adk_config = Configuration(
            name=config.name,
            description=config.description,
            model=config.model,
            temperature=config.temperature,
        )
        
        # Create the agent based on type
        if config.type == "llm":
            agent = LlmAgent(
                config=adk_config,
                system_prompt=config.system_prompt or "You are a helpful AI assistant.",
            )
        else:
            # For other agent types, use base Agent class
            agent = Agent(config=adk_config)
        
        # Add tools if specified
        if config.tools:
            for tool_name in config.tools:
                tool = tool_registry.get(tool_name)
                if tool:
                    agent.add_tool(tool)
        
        # Enable memory if requested
        if config.memory_enabled:
            agent.enable_memory(Memory())
        
        # Store agent in registry
        agent_registry[config.id] = agent
        
        return {
            "id": config.id,
            "status": "created",
            "message": f"Agent '{config.name}' created successfully"
        }
        
    except Exception as e:
        logger.error(f"Error creating agent: {str(e)}")
        raise HTTPException(status_code=500, detail=str(e))


@app.post("/agents/{agent_id}/execute")
async def execute_task(agent_id: str, request: TaskRequest):
    """Execute a task with an agent"""
    agent = agent_registry.get(agent_id)
    if not agent:
        raise HTTPException(status_code=404, detail=f"Agent '{agent_id}' not found")
    
    try:
        # Execute the task
        if request.stream:
            # For streaming responses
            async def stream_response():
                async for chunk in agent.stream(request.task, context=request.context):
                    yield json.dumps({"type": "chunk", "data": chunk}) + "\n"
                yield json.dumps({"type": "complete"}) + "\n"
            
            from fastapi.responses import StreamingResponse
            return StreamingResponse(stream_response(), media_type="text/event-stream")
        else:
            # For regular responses
            result = await agent.execute(request.task, context=request.context)
            return {
                "agent_id": agent_id,
                "task": request.task,
                "result": result
            }
            
    except Exception as e:
        logger.error(f"Error executing task: {str(e)}")
        raise HTTPException(status_code=500, detail=str(e))


@app.get("/agents")
async def list_agents():
    """List all registered agents"""
    return {
        "agents": [
            {
                "id": agent_id,
                "name": agent.config.name,
                "type": type(agent).__name__,
                "status": "active"
            }
            for agent_id, agent in agent_registry.items()
        ]
    }


@app.delete("/agents/{agent_id}")
async def delete_agent(agent_id: str):
    """Delete an agent"""
    if agent_id not in agent_registry:
        raise HTTPException(status_code=404, detail=f"Agent '{agent_id}' not found")
    
    del agent_registry[agent_id]
    return {"message": f"Agent '{agent_id}' deleted successfully"}


@app.post("/tools/register")
async def register_tool(config: ToolConfig):
    """Register a new tool"""
    try:
        # Create tool from configuration
        tool = Tool(
            name=config.name,
            description=config.description,
            parameters=config.parameters,
            # In real implementation, this would map to actual functions
            function=lambda **kwargs: {"result": f"Executed {config.name} with {kwargs}"}
        )
        
        tool_registry.register(tool)
        
        return {
            "name": config.name,
            "status": "registered",
            "message": f"Tool '{config.name}' registered successfully"
        }
        
    except Exception as e:
        logger.error(f"Error registering tool: {str(e)}")
        raise HTTPException(status_code=500, detail=str(e))


@app.get("/tools")
async def list_tools():
    """List all registered tools"""
    return {
        "tools": [
            {
                "name": tool.name,
                "description": tool.description,
                "parameters": tool.parameters
            }
            for tool in tool_registry.list()
        ]
    }


@app.post("/orchestrate")
async def orchestrate_agents(task: str, agent_ids: List[str]):
    """Orchestrate multiple agents for a complex task"""
    try:
        # Get agents from registry
        agents = [agent_registry[aid] for aid in agent_ids if aid in agent_registry]
        
        if not agents:
            raise HTTPException(status_code=404, detail="No valid agents found")
        
        # Create orchestrator
        orchestrator = Orchestrator(agents=agents)
        
        # Execute orchestrated task
        result = await orchestrator.execute(task)
        
        return {
            "task": task,
            "agents_used": agent_ids,
            "result": result
        }
        
    except Exception as e:
        logger.error(f"Error in orchestration: {str(e)}")
        raise HTTPException(status_code=500, detail=str(e))


@app.get("/models")
async def list_available_models():
    """List available models from Google AI"""
    return {
        "models": [
            {
                "id": "gemini-1.5-pro",
                "name": "Gemini 1.5 Pro",
                "context_window": 2097152,
                "capabilities": ["text", "code", "vision", "function_calling"]
            },
            {
                "id": "gemini-1.5-flash",
                "name": "Gemini 1.5 Flash",
                "context_window": 1048576,
                "capabilities": ["text", "code", "vision", "function_calling"]
            },
            {
                "id": "gemini-2.0-flash-exp",
                "name": "Gemini 2.0 Flash (Experimental)",
                "context_window": 1048576,
                "capabilities": ["text", "code", "vision", "function_calling", "real_time"]
            }
        ]
    }


def main():
    """Main entry point"""
    logger.info("Starting Google ADK Bridge Server...")
    uvicorn.run(app, host="0.0.0.0", port=8888)


if __name__ == "__main__":
    main()