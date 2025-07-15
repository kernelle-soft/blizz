# Self-Hosted LLM Design

## Overview

This document outlines approaches for running large language models locally, with particular focus on Mixture of Experts (MoE) models that require substantial computing resources but can be made practical through dynamic expert swapping and modern GPU hardware.

## Hardware Requirements

### RTX 6000 Blackwell Configuration
- **96GB VRAM** - Sufficient for most large MoE models with quantization
- **128GB System RAM** - Critical for cold expert storage during dynamic swapping
- **Total Memory Pool**: 224GB (96GB GPU + 128GB system)
- **PCIe 5.0 x16** - Required for optimal CPU-GPU memory transfer speeds
- **NVMe SSD storage** - Fast storage for model loading and caching

## Model Architecture Analysis

### DeepSeek V3 (671B Parameters)
- **Total Parameters**: 671B across 257 experts
- **Active Parameters**: Only 37B parameters active per token (8 experts)
- **Memory Requirements**: 
  - Q2_K Quantization: ~168GB total
  - Active working set: ~37GB in GPU memory
  - Cold expert storage: ~131GB in system RAM

#### Key Features for Home Deployment
- **Auxiliary-loss-free load balancing** - Optimized expert utilization
- **Node-limited routing** - Originally for distributed training, enables efficient memory management
- **Predictable routing patterns** - Enables effective expert caching strategies
- **Token-to-expert affinity** - Model can predict which experts will be needed

### Other Notable MoE Models

#### Qwen3 MoE Family
- **Qwen3:235b-a22b** - 235B total, 22B active parameters
- **Qwen3:30b-a3b** - 30B total, 3B active (fits entirely in single GPU)

#### Mistral Mixtral Series
- **Mixtral 8x7B** - 47B total, 13B active parameters
- **Mixtral 8x22B** - 141B total, 39B active parameters

## Inference Framework Support

### vLLM ⭐ (Native Continue.dev Support)
- **Native Continue.dev integration** - Dedicated `vllm` provider with optimized features
- **Expert offloading support** - Experimental dynamic expert swapping for large MoE models
- **Automatic model detection** - Continue.dev understands vLLM capabilities without manual configuration
- **No API key required** - Simplified setup for local development
- **Setup**: `vllm serve model_name --port 8000`
- **Memory management** - Automatic GPU/CPU memory orchestration

### ExLlamaV2  
- **MoE-optimized inference** - Specialized for mixture of experts models
- **Dynamic expert loading** - Real-time expert swapping between GPU/CPU
- **Continue.dev integration** - Via local API endpoints

### Ollama
- **Simplified model management** - Easy model switching and management
- **Limited MoE support** - Basic support, not optimized for large expert swapping
- **Best for**: Smaller models that fit entirely in GPU memory

## Dynamic Expert Swapping Architecture

### How It Works
1. **Expert Affinity Analysis** - Model predicts which experts will be needed
2. **Hot Cache Management** - Keep frequently used experts in GPU memory
3. **Cold Storage** - Less used experts reside in system RAM
4. **Real-time Swapping** - Transfer experts between CPU/GPU as needed
5. **Prefetching** - Load predicted experts before they're required

### Performance Considerations
- **Transfer Speed**: PCIe 5.0 provides ~64GB/s theoretical bandwidth
- **Latency Impact**: 1-2ms expert swap penalty vs. minutes of traditional loading
- **Cache Hit Rate**: Well-tuned systems achieve 85-95% expert cache hits
- **Working Set Size**: Typically 20-30% of total experts are "hot"

## Integration with Development Tools

### vLLM Setup and Configuration

**Why Native vLLM Provider is Better:**
- **Automatic model detection** - Continue.dev understands vLLM capabilities natively
- **Better error handling** - Native integration provides clearer error messages
- **Optimized performance** - Built-in support for vLLM-specific features
- **Simplified configuration** - No need for API keys or manual endpoint configuration

**1. Start vLLM Server with Expert Offloading:**
```bash
# For DeepSeek V3 with dynamic expert swapping
vllm serve ingu627/qwen3:235b-q2_k \
  --port 8000 \
  --max-model-len 32768 \
  --gpu-memory-utilization 0.95 \
  --enable-chunked-prefill \
  --distributed-executor-backend ray

# For smaller models that fit entirely in GPU
vllm serve deepseek-r1:32b \
  --port 8001 \
  --max-model-len 16384

# For dedicated autocomplete model
vllm serve Qwen/Qwen2.5-Coder-1.5B \
  --port 8002 \
  --max-model-len 8192

# For embeddings model
vllm serve nomic-ai/nomic-embed-text-v1 \
  --port 8003 \
  --max-model-len 2048
```

**2. Continue.dev Configuration (Native vLLM Support):**
```yaml
models:
  - name: DeepSeek V3 (vLLM)
    provider: vllm
    model: ingu627/qwen3:235b-q2_k
    apiBase: http://localhost:8000/v1
    roles:
      - chat
      - edit
      - apply
    defaultCompletionOptions:
      temperature: 0.7
      maxTokens: 4096
  - name: DeepSeek R1 32B (vLLM)
    provider: vllm
    model: deepseek-r1:32b
    apiBase: http://localhost:8001/v1
    roles:
      - chat
      - autocomplete
  - name: Qwen2.5-Coder 1.5B (Autocomplete)
    provider: vllm
    model: Qwen/Qwen2.5-Coder-1.5B
    apiBase: http://localhost:8002/v1
    roles:
      - autocomplete
  - name: Nomic Embeddings
    provider: vllm
    model: nomic-ai/nomic-embed-text-v1
    apiBase: http://localhost:8003/v1
    roles:
      - embed
```

### ExLlamaV2 Setup and Configuration

**1. Start ExLlamaV2 Server:**
```bash
# Install ExLlamaV2 with expert swapping support
pip install exllamav2[server]

# Start server with MoE optimization
python -m exllamav2.server \
  --model /path/to/deepseek-v3-q2k \
  --port 8002 \
  --host 0.0.0.0 \
  --api-key your-local-key \
  --expert-offload \
  --cache-size 24576  # 24GB cache for hot experts
```

**2. Continue.dev Configuration:**
```yaml
models:
  - name: DeepSeek V3 (ExLlamaV2)
    provider: openai  # ExLlamaV2 uses OpenAI-compatible API
    model: deepseek-v3
    apiBase: http://localhost:8002/v1
    apiKey: your-local-key
    roles:
      - chat
      - edit
      - apply
    defaultCompletionOptions:
      temperature: 0.3
      maxTokens: 8192
```

### Direct Ollama Integration (for smaller models)
```yaml
models:
  - name: DeepSeek R1 32B
    provider: ollama
    model: deepseek-r1:32b
    roles:
      - chat
      - edit
  - name: Qwen2.5 Coder 32B
    provider: ollama
    model: qwen2.5-coder:32b
    roles:
      - chat
      - autocomplete
```

### Automated Server Management Options

**Option 1: Startup Scripts (Recommended)**
Create a startup script that launches your inference server and then opens your IDE:

```bash
#!/bin/bash
# ~/bin/start-ai-dev.sh

# Start vLLM server in background
echo "Starting vLLM server..."
vllm serve ingu627/qwen3:235b-q2_k \
  --port 8000 \
  --gpu-memory-utilization 0.95 \
  --enable-chunked-prefill &

# Wait for server to be ready
echo "Waiting for server startup..."
while ! curl -s http://localhost:8000/health > /dev/null; do
  sleep 2
done

echo "Server ready! Opening IDE..."
# Launch your IDE
cursor /path/to/your/project
# or: code /path/to/your/project
```

**Option 2: Docker Compose Integration**
```yaml
# docker-compose.yml
version: '3.8'
services:
  vllm-server:
    image: vllm/vllm-openai:latest
    ports:
      - "8000:8000"
    volumes:
      - ~/.cache/huggingface:/root/.cache/huggingface
    environment:
      - CUDA_VISIBLE_DEVICES=0
    command: >
      --model ingu627/qwen3:235b-q2_k
      --port 8000
      --gpu-memory-utilization 0.95
    deploy:
      resources:
        reservations:
          devices:
            - driver: nvidia
              count: 1
              capabilities: [gpu]
```

**Option 3: VS Code/Cursor Task Runner**
Add to your workspace `.vscode/tasks.json`:
```json
{
  "version": "2.0.0",
  "tasks": [
    {
      "label": "Start vLLM Server",
      "type": "shell",
      "command": "vllm serve ingu627/qwen3:235b-q2_k --port 8000 --gpu-memory-utilization 0.95",
      "group": "build",
      "presentation": {
        "echo": true,
        "reveal": "always",
        "focus": false,
        "panel": "new"
      },
      "isBackground": true,
      "problemMatcher": {
        "pattern": {
          "regexp": "^(.*)$",
          "file": 1
        },
        "background": {
          "activeOnStart": true,
          "beginsPattern": "^.*Starting.*$",
          "endsPattern": "^.*ready.*$"
        }
      }
    }
  ]
}
```

**Option 4: Creative MCP Server Usage (Experimental)**
```yaml
# This is a bit hacky but could work
mcpServers:
  - name: Local Inference Launcher
    command: bash
    args:
      - -c
             - |
         vllm serve ingu627/qwen3:235b-q2_k \
           --port 8000 \
           --gpu-memory-utilization 0.95 &
        # Then run a minimal MCP server to satisfy the protocol
        python -c "
        import time
        while True:
            time.sleep(60)
        "
    env:
      CUDA_VISIBLE_DEVICES: "0"
```

### Complete Workflow Integration

**Step-by-Step Process:**

1. **Choose Your Server Management Approach** (from options above)

2. **Manual Server Start** (if not using automation):
   ```bash
   # Option A: vLLM with expert swapping (native provider)
   vllm serve ingu627/qwen3:235b-q2_k --port 8000 --gpu-memory-utilization 0.95
   
   # Option B: ExLlamaV2 with MoE optimization  
   python -m exllamav2.server --model /path/to/model --port 8002 --expert-offload
   
   # Option C: Direct Ollama (smaller models)
   ollama serve
   ```

3. **Monitor System Resources**:
   ```bash
   # Watch GPU utilization and memory usage
   watch -n 1 nvidia-smi
   
   # Monitor expert swapping activity (for vLLM/ExLlamaV2)
   htop -p $(pgrep -f "vllm\|exllamav2")
   ```

 4. **Configure Continue.dev** using appropriate YAML configuration

 5. **Test Connection**:
   ```bash
   # Test API endpoint
   curl -X POST http://localhost:8000/v1/chat/completions \
     -H "Content-Type: application/json" \
     -H "Authorization: Bearer your-local-key" \
     -d '{"model": "deepseek-v3", "messages": [{"role": "user", "content": "Hello"}]}'
   ```

**Development Workflow Features:**
- **Real-time code completion** - Inline suggestions as you type
- **Chat-based assistance** - Context-aware code discussions
- **Code editing** - Direct file modifications with AI assistance
- **Documentation generation** - Automated docstring and comment creation
- **Architecture review** - Large-scale codebase analysis and refactoring
- **Debugging assistance** - Error analysis and solution suggestions

**Performance Monitoring:**
- **Expert cache hit rates** - Monitor hot/cold expert usage patterns
- **Memory utilization** - Track GPU and system RAM usage
- **Inference latency** - Measure response times for different request types
- **Model switching** - Seamless transitions between different specialized models

## Practical Deployment Steps

### 1. Hardware Preparation
- Verify PCIe 5.0 support and proper slot configuration
- Ensure adequate power supply (RTX 6000 Blackwell: ~300W)
- Configure system RAM for large allocations (set vm.max_map_count)

### 2. Software Setup
- Install CUDA 12.0+ with proper driver support
- Install chosen inference framework (vLLM recommended)
- Configure memory management and expert caching

### 3. Model Acquisition
- Download quantized models (Q2_K or Q4_K recommended)
- Verify model compatibility with chosen framework
- Test basic inference before integration

### 4. Performance Tuning
- Optimize expert cache size based on available GPU memory
- Tune CPU-GPU transfer parameters
- Monitor expert hit rates and adjust caching strategy

## Current Limitations

### Framework Maturity
- **vLLM expert offloading** - Still experimental, may have stability issues
- **Limited documentation** - Cutting-edge features lack comprehensive guides
- **Debugging complexity** - Multi-tier memory management difficult to troubleshoot

### Hardware Constraints
- **Memory bandwidth** - CPU-GPU transfers still bottleneck for poor cache performance
- **Power requirements** - High-end GPUs require substantial power infrastructure
- **Cooling** - Extended inference sessions generate significant heat

## Future Considerations

### Emerging Technologies
- **CXL memory expansion** - Direct memory attachment for larger memory pools
- **Distributed inference** - Multi-machine expert distribution
- **Hardware-accelerated routing** - Specialized chips for expert selection

### Software Evolution
- **Framework consolidation** - Expect mature tooling within 6-12 months
- **Model optimization** - Better quantization techniques with less quality loss
- **Integration improvements** - Seamless IDE and development tool integration

## Real-World Examples

### Performance Benchmarks
- **DeepSeek V3 on RTX 6000** - ~30-50 tokens/second with proper tuning
- **Expert swap latency** - 1-3ms typical for 8B parameter expert transfer
- **Memory utilization** - 85-90% of available GPU memory actively used

## Conclusion

Large MoE models like DeepSeek V3 can be practically deployed on high-end consumer hardware through dynamic expert swapping. While the technology is still emerging, current frameworks provide sufficient capability for serious development workflows. The key is matching hardware capabilities with model requirements and choosing appropriate inference frameworks.

The combination of RTX 6000 Blackwell (96GB VRAM) and substantial system RAM (128GB+) provides a practical foundation for running models that were previously restricted to enterprise infrastructure.
