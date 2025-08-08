# SymbioteIDE - AI Master Tool

A revolutionary AI-powered desktop IDE combining code editing, visual development, multi-agent autonomous systems, and integration with 40+ AI providers including **OpenRouter**.

## 🚀 Features

### Multi-Provider AI Integration
- **OpenRouter** - Access to 40+ models through a single API (GPT-4o, Claude 3.5, Gemini 2.0, DeepSeek, Qwen, Llama, etc.)
- **OpenAI** - GPT-4, GPT-3.5-turbo
- **Anthropic** - Claude 3.5 Sonnet, Claude 3 Haiku
- **Google** - Gemini 2.0 Flash
- Intelligent model selection and cost optimization

### Core Systems
- **Custom Parser Engine** - AI-optimized AST with Tree-sitter for multiple languages
- **Multi-Agent System** - Architect, Developer, Tester, Reviewer, Orchestrator agents
- **Context Management** - Hierarchical context scopes with intelligent discovery
- **Zero-Error Development** - Checkpoint & rollback system with quality gates
- **Advanced Code Editor** - Monaco-based with AI assistance (explain, refactor, debug)
- **AI-Enhanced Terminal** - Multi-tab terminal with command prediction

## 🛠 Tech Stack

- **Backend**: Rust + Tauri 2.0
- **Frontend**: React + TypeScript + Vite
- **Editor**: Monaco Editor
- **Parsing**: Tree-sitter (Rust, TypeScript, JavaScript, Python)
- **Styling**: Custom CSS with GitHub Dark theme
- **Icons**: Lucide React

## 📦 Installation & Setup

### Prerequisites
- [Node.js](https://nodejs.org/) (v18+)
- [Rust](https://rustup.rs/) (latest stable)
- [Tauri CLI](https://tauri.app/v1/guides/getting-started/prerequisites)

### 1. Install Dependencies

```bash
# Navigate to project directory
cd symbiote-ide

# Install frontend dependencies
npm install

# Install Tauri CLI (if not already installed)
npm install -g @tauri-apps/cli@next

# Install Rust dependencies (automatically handled by Cargo)
```

### 2. Configure AI Providers

Create a `.env` file in the project root:

```env
# OpenRouter (Recommended - Access to 40+ models)
OPENROUTER_API_KEY=your_openrouter_api_key_here

# Optional: Direct provider keys
OPENAI_API_KEY=your_openai_api_key_here
ANTHROPIC_API_KEY=your_anthropic_api_key_here
GOOGLE_API_KEY=your_google_api_key_here
```

#### Getting OpenRouter API Key
1. Visit [OpenRouter.ai](https://openrouter.ai/)
2. Sign up for an account
3. Go to [API Keys](https://openrouter.ai/keys)
4. Create a new API key
5. Add credits to your account for usage

### 3. Run Development Server

```bash
# Start the development server
npm run tauri dev
```

This will:
- Start the Vite development server (frontend)
- Compile the Rust backend
- Launch the Tauri desktop application

### 4. Build for Production

```bash
# Build the application
npm run tauri build
```

## 🎯 Usage

### AI-Powered Code Editing
1. **Open a file** in the editor
2. **Select code** or place cursor
3. **Use AI features**:
   - `Ctrl+E` - AI Explain Code
   - `Ctrl+R` - AI Refactor Code
   - `Ctrl+D` - AI Debug Code
   - Or use the AI prompt input in the toolbar

### Multi-Agent System
1. **Create agents** using the Agent Panel
2. **Assign tasks** to specific agent types:
   - **Architect**: System design and planning
   - **Developer**: Code implementation
   - **Tester**: Testing and QA
   - **Reviewer**: Code review and analysis
   - **Orchestrator**: Multi-agent coordination

### Context Management
- **Symbols Panel**: View parsed code symbols and structure
- **Context Panel**: See current context scope and metadata
- **AST Viewer**: Inspect the abstract syntax tree

### Terminal Integration
- **Multi-tab terminal** with AI enhancement
- **Command prediction** and assistance
- **Integrated with IDE** for seamless workflow

## 🔧 Configuration

### AI Provider Selection
The default provider is set to **OpenRouter** with the `openai/gpt-4o` model for optimal performance and cost efficiency. You can modify this in:

- **Frontend**: `src/App.tsx` - `handleAIRequest` function
- **Backend**: `src-tauri/src/ai_provider.rs` - Provider initialization

### Available OpenRouter Models
- `openai/gpt-4o` - Latest GPT-4 Omni (recommended)
- `openai/gpt-4o-mini` - Faster, cheaper GPT-4
- `anthropic/claude-3.5-sonnet` - Claude 3.5 Sonnet
- `anthropic/claude-3-haiku` - Fast Claude model
- `google/gemini-2.0-flash-exp` - Google's latest Gemini
- `deepseek/deepseek-chat` - DeepSeek's coding model
- `qwen/qwen-2.5-72b-instruct` - Alibaba's Qwen model
- `meta-llama/llama-3.1-405b-instruct` - Meta's Llama 3.1
- `mistralai/mistral-large` - Mistral's large model
- `cohere/command-r-plus` - Cohere's command model

## 🏗 Architecture

### Backend (Rust)
```
src-tauri/src/
├── main.rs              # Main Tauri application
├── ai_provider.rs       # Multi-provider AI integration
├── parser.rs           # Custom parser with Tree-sitter
├── agent_system.rs     # Multi-agent system
└── context_manager.rs  # Context management
```

### Frontend (React/TypeScript)
```
src/
├── App.tsx                    # Main application
├── components/
│   ├── Editor.tsx            # Monaco code editor
│   ├── Sidebar.tsx           # File explorer
│   ├── Terminal.tsx          # AI-enhanced terminal
│   ├── AgentPanel.tsx        # Agent management
│   └── ContextPanel.tsx      # Context visualization
└── App.css                   # Styling
```

## 🚧 Development Roadmap

### Phase 1: Foundation (Months 1-3) ✅
- [x] Core infrastructure setup
- [x] Multi-provider AI integration (OpenRouter, OpenAI, Anthropic, Google)
- [x] Custom parser engine with Tree-sitter
- [x] Basic agent system foundation
- [x] Context management system
- [x] Advanced code editor with AI features

### Phase 2: Advanced IDE Systems (Months 4-6)
- [ ] Zero-error development system
- [ ] Advanced search & replace with semantic understanding
- [ ] Checkpoint & rollback system
- [ ] Enhanced context strategies
- [ ] Anti-duplication engine

### Phase 3: Multi-Agent & Automation (Months 7-9)
- [ ] Full multi-agent orchestration
- [ ] Task management system
- [ ] Browser automation
- [ ] Element inspection
- [ ] Knowledge management system

### Phase 4: Enterprise & Integration (Months 10-12)
- [ ] Diff/merge tools
- [ ] Native integrations
- [ ] Extension system
- [ ] Remote development
- [ ] Team collaboration features

## 🤝 Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Test thoroughly
5. Submit a pull request

## 📄 License

This project is licensed under the MIT License - see the LICENSE file for details.

## 🆘 Support

- **Documentation**: [Coming Soon]
- **Issues**: [GitHub Issues](https://github.com/symbiote-ide/symbiote-ide/issues)
- **Discord**: [Community Server](https://discord.gg/symbiote-ide)

## 🙏 Acknowledgments

- **Tauri** - For the amazing desktop app framework
- **Monaco Editor** - For the powerful code editor
- **Tree-sitter** - For the incremental parsing library
- **OpenRouter** - For unified access to multiple AI models
- **All AI providers** - For making this revolutionary IDE possible

---

**SymbioteIDE** - Where AI meets development. 🚀
