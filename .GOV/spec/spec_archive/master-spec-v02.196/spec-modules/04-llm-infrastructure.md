---
schema: handshake.indexed_spec.module@1
spec_version: "v02.196"
bundle_id: "master-spec-v02.196"
module_id: "04"
section_id: "4"
title: "4. LLM Infrastructure"
source_baseline_version: "v02.182"
source_baseline_path: ".GOV/spec/Handshake_Master_Spec_v02.182.md"
source_body_original_sha256: "3e96e64d7dbb7aac78a1f5218e57ad76230e151ccfa3126c294c169c039e3248"
body_sha256: "8c229a3551b62b4127dd4958fee27520f629d9a5cb7ffc27c6b77c071fb8dcaf"
metadata_rule: "frontmatter is machine metadata; body follows after this block"
---
# 4. LLM Infrastructure

## 4.1 LLM Infrastructure

**Why**  
Running AI models locally requires understanding how they work, how much resource they consume, and what trade-offs exist. This section provides the foundational knowledge for all model-related decisions.

**What**  
Explains how LLMs work at a practical level (parameters, inference vs training), key concepts (tokens, context windows, quantization, GGUF format), and sizing guidance for what fits on a 24GB RTX 3090.

**Jargon**  
- **LLM**: Large Language Modelâ€”AI that generates text by predicting the next word.
- **Parameters**: The "knowledge" of a model stored as numbers; more parameters = more capability but more memory.
- **Inference**: Using a trained model to generate outputs (we do inference, not training).
- **Token**: A chunk of text (~0.75 words); models think in tokens, not characters.
- **Context Window**: How many tokens a model can "see" at onceâ€”its working memory.
- **Quantization**: Compressing a model by reducing number precision (e.g., 16-bit â†’ 4-bit).
- **Q4/Q5/Q8**: Quantization levels; Q4 = smallest/fastest, Q8 = highest quality.
- **GGUF**: Standard file format for quantized local models (used by llama.cpp, Ollama).
- **KV Cache**: Memory used to store conversation context; grows with conversation length.

---

### 4.1.1 How LLMs Work (Simplified)

#### 4.1.1.1 The Basic Idea

**An LLM is a very sophisticated autocomplete.** Given some text, it predicts what text should come nextâ€”but it's so good at this that it can write essays, code, answer questions, and more.

```
You type:       "Write a haiku about programming"
                           â”‚
                           â–¼
                  â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
                  â”‚   LLM Model     â”‚
                  â”‚  (Billions of   â”‚
                  â”‚   parameters)   â”‚
                  â””â”€â”€â”€â”€â”€â”€â”€â”€â”¬â”€â”€â”€â”€â”€â”€â”€â”€â”˜
                           â”‚
                           â–¼
Model outputs:  "Code flows like water
                 Bugs emerge from the depths below
                 Debug, rinse, repeat"
```

#### 4.1.1.2 What "Parameters" Mean

Think of parameters as the model's "brain cells"â€”connections that store patterns learned from training data.

```
Model Size Guide:
â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
  3B-4B   â”‚  Small  â”‚  Fast, limited capability     
  7B-8B   â”‚  Medium â”‚  Good balance, our sweet spot 
  13B     â”‚  Large  â”‚  Better quality, slower       
  27B-30B â”‚  XL     â”‚  Near-GPT-3.5 quality         
  70B+    â”‚  XXL    â”‚  Best quality, very demanding 
â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
```

ðŸ’¡ **For our project:** 7B-13B models hit the sweet spot of quality vs. resource usage on a 3090.

---

### 4.1.2 Key Concepts: Tokens, VRAM, Quantization

#### 4.1.2.1 Understanding Tokens

**Tokens are how models measure text.** One token â‰ˆ 4 characters â‰ˆ 0.75 words.

```
Example tokenization:
"Hello, how are you today?" 
â†’ ["Hello", ",", " how", " are", " you", " today", "?"]
â†’ 7 tokens

Rough conversion:
  100 tokens  â‰ˆ 75 words   â‰ˆ 1 short paragraph
  1000 tokens â‰ˆ 750 words  â‰ˆ 1.5 pages
  4000 tokens â‰ˆ 3000 words â‰ˆ 6 pages
```

ðŸ“Œ **Why tokens matter:** 
- Models have a maximum context window (e.g., 4096 or 8192 tokens)
- Cloud APIs charge per token
- More tokens = slower responses and more memory

#### 4.1.2.2 Understanding Context Windows

**The context window is the model's "working memory."** It includes BOTH your prompt AND the model's response.

```
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚              4096 TOKEN CONTEXT WINDOW                  â”‚
â”œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¤
â”‚  System prompt (instructions)     â”‚  ~200 tokens       â”‚
â”‚  Conversation history             â”‚  ~2000 tokens      â”‚
â”‚  Current user message             â”‚  ~300 tokens       â”‚
â”‚  â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¼â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”‚
â”‚  Space for model's response       â”‚  ~1596 tokens      â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
```

âš ï¸ **Warning:** Long conversations eventually "forget" earlier messages when context fills up.

#### 4.1.2.3 Understanding Quantization

**Quantization shrinks models by reducing number precision.** Like saving a photo as JPEG instead of RAWâ€”smaller file, slight quality loss.

```
â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
                    CORE CONCEPT: QUANTIZATION
â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
  
  Original model: 7B parameters at 16-bit = ~14 GB
  
  Quantized versions:
  â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¬â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¬â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¬â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
  â”‚ Format   â”‚ Bits     â”‚ Size        â”‚ Quality Loss       â”‚
  â”œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¼â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¼â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¼â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¤
  â”‚ Q8_0     â”‚ 8-bit    â”‚ ~7 GB       â”‚ Minimal (<1%)      â”‚
  â”‚ Q5_K_M   â”‚ 5-bit    â”‚ ~5 GB       â”‚ Very small (~1-2%) â”‚
  â”‚ Q4_K_M   â”‚ 4-bit    â”‚ ~4 GB       â”‚ Small (~2-3%)      â”‚ â† Sweet spot
  â”‚ Q3_K_M   â”‚ 3-bit    â”‚ ~3 GB       â”‚ Noticeable (~5%)   â”‚
  â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”´â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”´â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”´â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
  
  ðŸ“Œ Q4_K_M is the most common choice: good quality, big savings

â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
```

ðŸ’¡ **For our project:** We'll primarily use Q4_K_M quantized models in GGUF format.

#### 4.1.2.4 VRAM Usage: Putting It Together

```
Formula for VRAM estimate:
  VRAM â‰ˆ (Parameters in billions) Ã— (Bits Ã· 2) GB
  
  Examples with Q4 (4-bit):
  â€¢ 7B model:  7 Ã— (4Ã·2) = 7 Ã— 2 = ~3.5-4 GB
  â€¢ 13B model: 13 Ã— (4Ã·2) = 13 Ã— 2 = ~6.5-8 GB  
  â€¢ 70B model: 70 Ã— (4Ã·2) = 70 Ã— 2 = ~35 GB... but actually fits in ~17-18GB 
                          (due to efficient formats)
```

---

### 4.1.3 Model Sizes and What Fits

#### 4.1.3.1 Quick Reference Table

| Model Size | Quantization | VRAM Needed | Speed (tokens/sec) | Quality Level |
|------------|--------------|-------------|-------------------|---------------|
| 3-4B | Q4 | ~2-3 GB | 60-200 | Basic tasks |
| 7-8B | Q4 | ~4-5 GB | 50-130 | Good general use |
| 13B | Q4 | ~7-9 GB | 30-70 | Very good |
| 27B | Q4 | ~14 GB | 20-30 | Excellent |
| 70B | Q4 | ~17-18 GB | 10-15 | Near GPT-3.5 |

#### 4.1.3.2 What Fits on Our 24GB RTX 3090?

```
Scenario Planning for 24 GB VRAM:
â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

âœ“ COMFORTABLE (with headroom):
  â€¢ 3Ã— 7B models (12 GB) + context buffer
  â€¢ 2Ã— 13B models (16 GB) + some headroom  
  â€¢ 1Ã— 7B + 1Ã— 13B + 1Ã— 4B (15 GB)

âš¡ TIGHT (works but careful):
  â€¢ 1Ã— 70B model (17-18 GB) alone
  â€¢ 1Ã— 27B + 1Ã— 7B (18 GB)
  â€¢ 2Ã— 7B + SDXL image generation (8 + 10 = 18 GB)

âœ— WON'T FIT:
  â€¢ 2Ã— 70B models (34+ GB)
  â€¢ 70B + any substantial other model
  â€¢ Multiple 27B+ models
```

#### 4.1.3.3 The Speed Difference: GPU vs CPU

âš¡ **Critical:** Running models from GPU VRAM is approximately 6x faster than running them from system RAM.

| Where Model Lives | Speed | When to Use |
|-------------------|-------|-------------|
| GPU VRAM | ~50-130 tokens/sec | Always prefer this |
| System RAM (CPU) | ~8-20 tokens/sec | Last resort / fallback |

---

**Key Takeaways**  
- LLMs predict "what text comes next" so well they seem intelligent; we do inference, not training.
- Tokens â‰ˆ 0.75 words; context window limits total conversation length.
- Quantization (Q4/Q5) shrinks models 3-4Ã— with minimal quality loss; GGUF is the standard format.
- 7B Q4 model â‰ˆ 4GB VRAMâ€”this is our planning baseline.
- On 24GB VRAM: 2-3 small models (7B-13B) comfortably, or one 70B model alone.
- GPU is ~6Ã— faster than CPU; avoid CPU fallback for user-facing tasks.

---
**Key Takeaways (10.2 - Tokens/VRAM/Quantization)**
- Tokens â‰ˆ 0.75 words; context window limits total conversation length
- Quantization (Q4/Q5) shrinks models 3-4x with minimal quality loss
- GGUF is the standard format for local quantized models
- 7B Q4 model â‰ˆ 4GB VRAM; this is our planning baseline

## 4.2 LLM Inference Runtimes

**Why**  
The runtime software determines how efficiently models execute, how many requests can be handled concurrently, and how easily models can be managed. This section guides runtime selection.

**What**  
Defines what an inference runtime does, compares major open-source implementation options (llama.cpp, Candle, mistral.rs, vLLM, TGI, Ollama, LM Studio, llamafile), and requires a Handshake-native ModelRuntime strategy for core operation. External model-server daemons are compatibility-only opt-ins.

**Jargon**  
- **Runtime**: Software that loads and runs AI models.
- **API**: Application Programming Interfaceâ€”how our app communicates with the runtime.
- **OpenAI-compatible API**: An API matching OpenAI's format, so code written for ChatGPT works locally.
- **Streaming**: Sending response tokens one at a time as generated (better UX).
- **Batching**: Processing multiple requests together for efficiency.
- **Continuous Batching**: Advanced batching that dynamically adds/removes requests mid-generation.
- **PagedAttention**: vLLM's memory optimization technique for efficient KV cache management.

---

### 4.2.1 What is an Inference Runtime?

#### 4.2.1.1 The Role of an Inference Runtime

**A runtime is the software layer between your application and the AI model.** It handles:

```
Your App                    Runtime                     GPU
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”    HTTP API    â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”   CUDA/GPU    â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚ "Write  â”‚ â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€>â”‚ â€¢ Load   â”‚ â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€> â”‚ Matrix  â”‚
â”‚  me a   â”‚                â”‚   model  â”‚              â”‚ math on â”‚
â”‚  poem"  â”‚ <â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”‚ â€¢ Run    â”‚ <â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€ â”‚ tensors â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜   Streaming    â”‚   infer  â”‚              â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
              Response     â”‚ â€¢ Manage â”‚
                          â”‚   memory â”‚
                          â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
```

#### 4.2.1.2 Why Runtime Choice Matters

Different runtimes optimize for different things:

| Priority | Best Runtime | Trade-off |
|----------|-------------|-----------|
| Core operation | Handshake ModelRuntime | Handshake owns lifecycle, health, logs, and receipts |
| Quantized local models | LlamaCppRuntime | Native integration work required |
| Hook/custom-kernel research | CandleRuntime | Narrower model coverage |
| Compatibility import | ExternalEngineImport adapter | Explicit operator-configured adapter only |

---

### 4.2.2 Runtime Comparison: Handshake ModelRuntime vs Compatibility Engines [UPDATED v02.190]

Note: Benchmark numbers in this section (tokens/sec, VRAM budgets) require refresh against current 2026 model builds/quantization strategies and target hardware; update tables and guidance accordingly.

#### 4.2.2.1 Overview Table

```
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¬â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¬â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¬â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¬â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚ Runtime     â”‚ Ownership   â”‚ Performance  â”‚ Ease of Use   â”‚ Best For      â”‚
â”œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¼â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¼â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¼â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¼â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¤
â”‚ ModelRuntimeâ”‚ Handshake   â”‚ High         â”‚ â­â­â­â­ Managedâ”‚ Core runtime â”‚
â”‚ LlamaCpp    â”‚ Handshake   â”‚ Good         â”‚ â­â­â­ Medium   â”‚ Quantized    â”‚
â”‚ Candle      â”‚ Handshake   â”‚ Good         â”‚ â­â­â­ Medium   â”‚ Custom hooks â”‚
â”‚ ExternalImportâ”‚ Operator  â”‚ varies       â”‚ â­â­ Adapter   â”‚ Compat only â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”´â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”´â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”´â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”´â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
```

#### 4.2.2.2 Ollama â€” Compatibility Reference Only [UPDATED v02.190]

**What it is:** A user-friendly CLI tool and server for running local LLMs. This is legacy/reference context only. Ollama is not a Handshake ModelRuntime authority path and must not be required for core Handshake operation.

```bash
# Compatibility adapter example only; not required for core Handshake operation.
ollama run mistral

```
# Compatibility adapter example only; Handshake must not depend on this daemon.
ollama serve
```
# External localhost calls require explicit ExternalEngineImport configuration.
```

**Pros:**
- â­ Incredibly easy to set up (one-line install)
- â­ Built-in model management (download, update, delete)
- â­ OpenAI-compatible API out of the box
- â­ Automatic GPU/CPU fallback
- â­ Supports multiple models (swaps them in/out of VRAM)

**Cons:**
- âš ï¸ Not optimized for high concurrency (~41 tokens/sec under load vs vLLM's ~793)
- âš ï¸ No advanced batching (processes one request fully before next)
- âš ï¸ Model switching has latency (unload/load takes seconds)

**Performance Numbers:**
```
Ollama on RTX 3090 (single user):
  â€¢ Mistral-7B Q4:  ~100-130 tokens/sec
  â€¢ Llama2-13B Q4:  ~40-50 tokens/sec
  â€¢ Under heavy load: drops to ~41 tokens/sec (no batching)
```

**Best for:** Development, personal use, low-concurrency production

---

### 4.2.3 LLM Client Adapter (Normative)

To satisfy the **Single Client Invariant [CX-101]**, all application code MUST interact with LLMs through the `LlmClient` trait. This ensures provider portability and centralized observability.

#### 4.2.3.1 LlmClient Trait

```rust
/// HSK-TRAIT-004: LLM Client Adapter
#[async_trait]
pub trait LlmClient: Send + Sync {
    /// Executes a completion request.
    /// Returns:
    /// - Ok(CompletionResponse): The generated text and usage metadata.
    /// - Err(LlmError): If the request fails or budget is exceeded.
    async fn completion(
        &self, 
        req: CompletionRequest
    ) -> Result<CompletionResponse, LlmError>;

    /// Returns the model profile (capabilities, token limits).
    fn profile(&self) -> &ModelProfile;
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CompletionRequest {
    pub trace_id: Uuid,          // REQUIRED: non-nil
    pub prompt: String,
    pub model_id: String,
    pub max_tokens: Option<u32>,
    pub temperature: f32,
    pub stop_sequences: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CompletionResponse {
    pub text: String,
    pub usage: TokenUsage,
    pub latency_ms: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Debug, ThisError)]
pub enum LlmError {
    #[error("HSK-429-RATE-LIMIT: Provider rate limit exceeded")]
    RateLimit,
    #[error("HSK-402-BUDGET-EXCEEDED: Token budget exceeded: {0}")]
    BudgetExceeded(u32),
    #[error("HSK-500-LLM: Internal provider error: {0}")]
    ProviderError(String),
}
```

#### 4.2.3.1.1 Traceability Addendum (Normative)

To satisfy the traceability and observability requirements, every LLM completion MUST be attributable to a non-nil `trace_id`.

Normative requirement: the LLM completion request MUST include `trace_id` used for Flight Recorder correlation.

Kernel V1 authority boundary [ADD v02.184]: model runtime traces, provider request IDs, framework tracing spans, and Flight Recorder correlation IDs are observability surfaces. They are not Kernel V1 authority. A Kernel V1 model call that participates in session execution MUST be linked to a durable `SessionRun`, `KernelTaskRun`, ContextBundle, ModelAdapter invocation, and EventLedger event chain. Replay, promotion, and validation decisions MUST be reconstructable from product-owned durable state even when provider-side trace history is absent.

```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct CompletionRequest {
    pub trace_id: Uuid,          // REQUIRED: non-nil
    pub prompt: String,
    pub model_id: String,
    pub max_tokens: Option<u32>,
    pub temperature: f32,
    pub stop_sequences: Vec<String>,
}
```

#### 4.2.3.2 Implementation Requirements

Kernel V1 adapter addendum [ADD v02.184]: Kernel V1 MUST call providers through a replaceable ModelAdapter boundary. The first proof MUST include a deterministic local dummy adapter so kernel session, tool, artifact, validation, and promotion flows can be tested without provider availability.

1.  **Handshake-native adapter:** The primary implementation for Phase 1 MUST use the Handshake ModelRuntime boundary with deterministic local dummy proof plus native/runtime-discovered engines. Ollama API support, if present, is an explicit compatibility adapter and MUST NOT be the default or a proof prerequisite.
2.  **Budget Enforcement:** The client MUST enforce `max_tokens` and return `BudgetExceeded` if the provider exceeds the limit.
3.  **Observability:** Every call MUST emit a Flight Recorder event (Â§11.5) containing `trace_id`, `model_id`, and `TokenUsage`.

---

### vLLM â€” Reference Architecture / Compatibility Adapter [UPDATED v02.190]

**What it is:** A high-performance inference engine from UC Berkeley, optimized for throughput. In Handshake it is reference architecture or an explicit ExternalEngineImport/operator-configured adapter, not a core model-server prerequisite.

```bash
# Compatibility adapter example only; not a core Handshake runtime.
python -m vllm.entrypoints.openai.api_server \
  --model mistralai/Mistral-7B-v0.1 \
  --port 8000
```

**Pros:**
- â­ Extremely fast: ~793 tokens/sec under load (vs Ollama's ~41)
- â­ Continuous batching: efficiently handles many concurrent requests
- â­ PagedAttention: optimizes memory usage
- â­ Scales almost linearly with more requests
- â­ OpenAI-compatible API

**Cons:**
- âš ï¸ One model per process (need multiple processes for multiple models)
- âš ï¸ More complex setup than Ollama
- âš ï¸ GPU-only (no CPU fallback)
- âš ï¸ Python-based (adds some overhead to embed)

**Performance Numbers:**
```
vLLM on RTX 3090:
  â€¢ Single request:   Similar to Ollama
  â€¢ 10 concurrent:    ~793 tokens/sec total (vs Ollama's ~41)
  â€¢ Scales to 100s of concurrent requests efficiently
```

**Best for:** Reference design and explicit compatibility imports where a user already operates vLLM; Handshake core production uses ModelRuntime-owned engines.

#### 4.2.2.3 HuggingFace TGI (Text Generation Inference) [UPDATED v02.190]

**What it is:** HuggingFace's production-grade inference server, used in their cloud offerings. In Handshake it is reference/compatibility context only; core local-model operation must use Handshake-native ModelRuntime, product-managed subprocesses, or in-process libraries.

```bash
# Reference only; do not use as a Handshake core-runtime setup.
# External TGI/Docker launch is compatibility-only and requires explicit operator opt-in.
```

**Pros:**
- â­ Production-tested at scale (powers HuggingFace Inference Endpoints)
- â­ Continuous batching like vLLM
- â­ Built-in metrics (Prometheus) and tracing
- â­ Supports many quantization formats (GPTQ, AWQ, bitsandbytes)
- â­ OpenAI-compatible API

**Cons / compatibility caveats [UPDATED v02.190]:**
- One model per external server/container in the upstream TGI pattern.
- External TGI/Docker operation is compatibility-only and requires explicit operator opt-in; it is not a Handshake core-runtime prerequisite.
- Heavier setup than Handshake-native ModelRuntime.

**Best for:** Enterprise production, when you need built-in observability

#### 4.2.2.4 Other Options (Brief)

**LM Studio:**
- GUI application for exploring models
- Has a server mode with OpenAI API
- Great for testing, not ideal for production automation
- Closed-source

**llamafile:**
- Single executable per model (bundles model + runtime)
- Just download and runâ€”no installation
- Limited features, single-threaded
- Best for distributing a pre-packaged model to end users

**llama.cpp (via Python bindings):**
- The engine under Ollama and many others
- Can embed directly in your code
- More control, more complexity
- Good for custom integrations

---

### 4.2.4 Recommended Runtime Strategy

**[REWRITTEN v02.186 — Ollama-as-primary recommendation removed per KERNEL-004 invariant §3.6.]**

The pre-Kernel-V1 version of this subsection recommended Ollama as the Phase-1 primary local runtime, with vLLM bolted on for heavy/batch loads. That recommendation is retired in v02.186. KERNEL-004 makes local-model runtime an in-process Handshake concern; no third-party model-server daemon is an authoritative runtime under the new architecture.

Local runtime strategy is governed by **§4.6 ModelRuntime + LocalModelAdapter**, which defines the normative path:

- `LlamaCppRuntime` (DEFAULT) via the `llama-cpp-2` Rust crate — covers ~all GGUF transformer models.
- `CandleRuntime` via `candle-core` + `candle-transformers` — required for activation-hook-requiring techniques (steering, refusal-vector, CAA) and for subquadratic architectures (Mamba2 / RWKV).
- Every local model is boxed inside a `SandboxAdapter` (§3.5) and tracked in `ProcessOwnershipLedger` (§5.7) per the §3.6 invariant.

The runtime-strategy comparison framing — fast lane vs. batch lane vs. specialized lane — remains valid; what changes is the *engines that fill each lane*. Operator may still opt into `ExternalEngineImport` (§3.6.4) to point Handshake at an out-of-band local OpenAI-compatible endpoint for compatibility/experiment, but those endpoints are not authoritative ModelRuntime instances.

```
=======================================================================
                 DECISION POINT: Runtime Strategy (v02.186)
=======================================================================

ALL LANES -> Handshake ModelRuntime (§4.6)
                |
                +-- LlamaCppRuntime         (default; GGUF transformers)
                +-- CandleRuntime           (steering hooks + subquadratic)
                +-- ExternalEngineImport    (compat-only; not authority)

Lane selection is per-model + per-Work-Profile; it is NOT a function of
which third-party app is installed on the host.
=======================================================================
```

#### 4.2.4.1 Integration Pattern

Routing logic lives inside `LlmClient` (§4.2.3, CX-101) and delegates to `ModelRuntime` (§4.6) for any `provider="local"` target. The pre-v02.186 sample that branched on `call_ollama(...)` vs `call_vllm(...)` is removed; in-process adapter dispatch is the new pattern:

```rust
// Conceptual: routing happens inside LlmClient -> ModelRuntime
match request.kind {
    RequestKind::InteractiveChat   => runtime.generate_stream(model, prompt, fast_profile),
    RequestKind::BatchProcess      => runtime.generate_stream(model, prompt, batch_profile),
    RequestKind::CodeGeneration    => runtime.generate_stream(code_model, prompt, fast_profile),
    _                              => runtime.generate_stream(model, prompt, default_profile),
}
```

The adapter behind `runtime` (LlamaCppRuntime vs CandleRuntime) is chosen at model-register time per §4.6.5.

---

**Key Takeaways (v02.186)**
- Runtime = software that loads and runs models; all major *third-party* options support OpenAI-compatible APIs, but Handshake's authoritative path is **in-process** via ModelRuntime (§4.6).
- llama.cpp remains the dominant low-level engine; Handshake consumes it through the `llama-cpp-2` Rust crate (LlamaCppRuntime), NOT through an external Ollama daemon.
- vLLM / TGI remain valid reference points for *cloud-grade* throughput characteristics; KERNEL-004 does not ship a vLLM adapter and is not blocked on one.
- Legacy Phase-0 / pre-Kernel-V1 work that depended on an external Ollama process is retired; see §3.6 (LocalModel Process Boxing Invariant) and §4.6 for the replacement architecture.

**Cross-references:** §3.6, §4.6, §4.2.3 LlmClient, CX-101 HARD_LLM_CLIENT, CX-102.

## 4.3 Model Selection & Roles

**Why**  
Using specialized models for specific tasks outperforms one large generalist, especially on constrained hardware. This section guides model selection for each role.

**What**  
Explains why specialized models beat generalists, defines role categories (orchestrator, code, creative, utility), recommends specific models for each role, and covers GPU memory management and scheduling strategies.

**Jargon**  
- **Orchestrator Model**: General-purpose model for reasoning, routing, and conversation.
- **Code Model**: Model fine-tuned specifically for programming tasks.
- **Creative Model**: Model optimized for long-form writing and creative generation.
- **Utility/Fast Model**: Small, fast model for classification, extraction, and simple tasks.
- **Hot Model**: A model kept loaded in VRAM for instant response.
- **On-Demand Model**: A model loaded only when specifically needed.
- **KV Cache**: Memory storing conversation context; grows with conversation length.

---

### 4.3.1 Specialized Models for Different Tasks

#### 4.3.1.1 Why Not One Model for Everything?

**Specialized models outperform generalists at specific tasks while using less resources.**

```
Analogy: Hiring Staff

Option A: One expensive expert who does everything "pretty well"
  â””â”€â”€ 70B generalist model (17GB VRAM, slow)

Option B: Team of specialists, each excellent at their job
  â””â”€â”€ 7B code model (4GB) + 7B chat model (4GB) + 7B creative (4GB)
  â””â”€â”€ Total: 12GB, all running simultaneously, each faster at their specialty

For our project: Option B is better
```

#### 4.3.1.2 Role Categories

| Role | What It Does | Characteristics Needed |
|------|--------------|------------------------|
| **Orchestrator** | General reasoning, routing decisions, conversation | Fast, good instruction-following |
| **Code Assistant** | Writing and explaining code | Trained on code, good at syntax |
| **Creative Writer** | Long-form content, stories, marketing | Larger context, creative outputs |
| **Utility/Fast** | Simple tasks: classification, extraction, yes/no | Tiny, extremely fast |

---

### 4.3.2 Model Recommendations by Role

#### 4.3.2.1 Orchestrator / General Purpose

**Primary Pick: Mistral-7B**

```
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚  MISTRAL-7B (Q4_K_M)                                       â”‚
â”œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¤
â”‚  Parameters:  7.3B                                         â”‚
â”‚  VRAM:        ~4.1 GB                                      â”‚
â”‚  Speed:       ~130 tokens/sec on 3090                      â”‚
â”‚  Context:     4K tokens (limited) or 8K with some variantsâ”‚
â”œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¤
â”‚  Strengths:                                                â”‚
â”‚    â€¢ Outperforms Llama2-13B despite being smaller         â”‚
â”‚    â€¢ Excellent instruction following                       â”‚
â”‚    â€¢ Very fast inference                                   â”‚
â”‚  Weaknesses:                                               â”‚
â”‚    â€¢ 4K context can be limiting for long conversations    â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
```

**Alternative: Llama2-13B** (when you need more capability or longer context)
- ~9 GB VRAM, ~40-50 tokens/sec
- 8K context window
- Better for complex reasoning

#### 4.3.2.2 Code Generation

**Primary Pick: CodeLlama-7B**

```
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚  CODELLAMA-7B (Q4_K_M)                                     â”‚
â”œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¤
â”‚  Parameters:  7B                                           â”‚
â”‚  VRAM:        ~3.8 GB                                      â”‚
â”‚  Speed:       ~100 tokens/sec on 3090                      â”‚
â”‚  Context:     16K tokens                                   â”‚
â”œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¤
â”‚  Strengths:                                                â”‚
â”‚    â€¢ Fine-tuned specifically for code                      â”‚
â”‚    â€¢ Supports Python, JS, C++, and more                   â”‚
â”‚    â€¢ Large context for reading whole files                â”‚
â”‚  Weaknesses:                                               â”‚
â”‚    â€¢ Less capable at general conversation                  â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
```

**Alternatives:**
- **StarCoder-7B:** Open-source, 16K context
- **WizardCoder-15B:** Higher quality (~8-9GB), better for complex tasks

#### 4.3.2.3 Creative / Long-Form Writing

**Primary Pick: Llama2-13B or Mistral-7B**

For most creative tasks, the orchestrator model works fine. For serious long-form writing:

**Consider: Llama2-70B (4-bit)** â€” Best quality, but uses ~17-18GB
- Only load when specifically needed for creative work
- Unload other models first
- ~15 tokens/sec (slower but higher quality)

#### 4.3.2.4 Utility / Fast Tasks

**Primary Pick: Phi-4 Mini (3.8B) or Gemma-3-4B**

```
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚  SMALL UTILITY MODELS                                      â”‚
â”œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¤
â”‚  Phi-4 Mini (3.8B Q4):    ~2.5 GB, ~60 tokens/sec         â”‚
â”‚  Gemma-3 4B (4-bit):      ~2.6 GB, ~200+ tokens/sec       â”‚
â”‚  Qwen2.5-3B (Q4):         ~2-3 GB, ~40 tokens/sec         â”‚
â”œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¤
â”‚  Use for:                                                  â”‚
â”‚    â€¢ Classification ("is this spam?")                      â”‚
â”‚    â€¢ Extraction ("find the date in this text")            â”‚
â”‚    â€¢ Simple Q&A                                            â”‚
â”‚    â€¢ Routing decisions                                     â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
```

#### 4.3.2.5 Recommended Starting Configuration

```
â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
                    DECISION POINT: Initial Model Setup
â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•

RECOMMENDED: "Always Hot" + "On-Demand" Strategy

Always Loaded ("Hot"):
  â”œâ”€â”€ Mistral-7B (4GB)      â†’ General orchestrator, fast chat
  â””â”€â”€ CodeLlama-7B (4GB)    â†’ Code assistance
  Total: ~8 GB (leaves 14GB free)

Load On-Demand:
  â”œâ”€â”€ Llama2-13B (9GB)      â†’ Complex reasoning when needed
  â”œâ”€â”€ Llama2-70B (17GB)     â†’ Best quality (swap out others first)
  â””â”€â”€ SDXL (7-10GB)         â†’ Image generation

Rationale:
  â€¢ Two 7B models handle 90% of tasks
  â€¢ Fast switching between chat and code
  â€¢ Load larger models only for complex work
  â€¢ Preserves VRAM for image generation

â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
```

---

### 4.3.3 GPU Memory Management

#### 4.3.3.1 The Loading Problem

**Models must be in VRAM to run fast.** Loading a model takes time:
- 7B model: ~3-5 seconds
- 13B model: ~5-10 seconds  
- 70B model: ~15-30 seconds

This creates a user experience challenge: if users request a model that isn't loaded, they wait.

#### 4.3.3.2 Strategies

**1. Keep "Hot" Models Resident**
```
Always keep your most-used models in VRAM:
  â€¢ Set Handshake ModelRuntime residency policy: max_loaded_models=2
  â€¢ These stay loaded even when idle
  â€¢ Instant response for common tasks
```

**2. On-Demand Loading with Feedback**
```
When user needs a different model:
  â€¢ Show loading indicator: "Loading creative writing model..."
  â€¢ Expected wait: 5-15 seconds
  â€¢ Consider preloading if you can predict need
```

**3. Never Use CPU Fallback for Primary Tasks**
```
CPU inference is ~6x slower:
  â€¢ GPU: 100 tokens/sec
  â€¢ CPU: ~15 tokens/sec
  
Only use CPU for:
  â€¢ Truly background tasks
  â€¢ When GPU is fully occupied with priority work
  â€¢ Emergency fallback (better slow than nothing)
```

#### 4.3.3.3 KV Cache: The Hidden Memory User

**Context uses extra VRAM beyond model weights.**

```
VRAM breakdown for a 7B model with long conversation:

  Model weights:        ~4 GB
  KV cache (context):   +2-4 GB for 4K tokens
  â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
  Total:                ~6-8 GB actual usage

âš ï¸ Long conversations can DOUBLE your VRAM usage!
```

ðŸ’¡ **Tip:** For multi-model setups, keep conversations shorter or implement context summarization.

---



#### 4.3.3.4 Sequential Model Swaps (Runtime Contract) (Normative) [ADD v02.120]

This subsection specifies the **required** model resource management behavior for Handshake runtimes operating under constrained GPU VRAM (single GPU, limited VRAM) where multiple specialized models must be **swapped in/out** of memory.

##### 4.3.3.4.1 Design Rationale (Informative)

- Handshake runs on constrained GPUs and expects multiple specialized models (orchestrator, coder, validator, vision).
- Many deployments cannot keep all models in VRAM simultaneously.
- The runtime MUST support **sequential model loading** with **state persistence** and **fresh context recompile** on resume.
- A model swap is a first-class runtime operation with traceability, budgets, and timeouts.

##### 4.3.3.4.2 Model Roles (Normative)

Handshake uses the following **runtime model roles** (distinct from governance mode):

| Role | Responsibilities | Typical Model Type |
|------|------------------|-------------------|
| `frontend` | Fast chat UX, intent capture, summarization, light reasoning | Small/medium local or cloud |
| `orchestrator` | Planning, routing, gate decisions, macro-task decomposition | Large reasoning model (local/cloud) |
| `worker` | Micro-task execution, code generation, transformations | Coding model (local-first) |
| `validator` | Verification, critique, policy checks, test/QA focus | Strong verifier model (local/cloud) |

Implementations MUST treat `role` as a **routing hint** (used by Work Profiles and escalation policies), not as an authority boundary.

##### 4.3.3.4.3 ModelSwapRequest (Normative)

A model swap request is a **typed, auditable** runtime command. Canonical JSON shape:

```typescript
export interface ModelSwapRequest {
  schema_version: "hsk.model_swap@0.4";
  request_id: string;

  // Current and target models
  current_model_id: string;
  target_model_id: string;

  // Role context (orchestrator/worker/validator/frontend)
  role: "frontend" | "orchestrator" | "worker" | "validator";

  // Priority and reason
  priority: "low" | "normal" | "high" | "critical";
  reason: string;   // e.g. "escalation", "profile_switch", "context_overflow"

  // Swap strategy (required)
  swap_strategy: "unload_reload" | "keep_hot_swap" | "disk_offload";

  // State persistence contract
  state_persist_refs: string[];  // Artifact refs (Locus checkpoint, MT state, etc.)
  state_hash: string;            // Hash of persisted state

  // Fresh context compilation requirement
  context_compile_ref: string;   // Reference to ACE context compilation job

  // Resource budgets
  max_vram_mb: number;
  max_ram_mb: number;
  timeout_ms: number;

  // Who requested the swap
  requester: {
    subsystem: "mt_executor" | "governance" | "ui" | "orchestrator";
    job_id?: string;
    wp_id?: string;
    mt_id?: string;
  };

  // Optional metadata
  metadata?: Record<string, any>;
}
```

##### 4.3.3.4.4 Model Swap Protocol (Normative)

When a `ModelSwapRequest` is issued, the runtime MUST execute the following steps:

1. **Persist state**: Flush and persist all state required to resume work:
   - Current Work Packet / MT state
   - Locus checkpoint
   - Partial outputs and file modifications (if any)
   - Governance gate state / pending approvals
2. **Emit event**: Log `FR-EVT-MODEL-001` (swap requested), including `reason` and correlation IDs.
3. **Unload/offload current model** per strategy:
   - If `unload_reload`, unload from VRAM entirely.
   - If `disk_offload`, write weights to disk if supported.
   - If `keep_hot_swap`, keep in VRAM only if budgets allow.
4. **Load target model**, respecting `max_vram_mb` and `max_ram_mb`.
5. **Recompile context**: Invoke ACE Runtime context compilation (Â§2.6.6.7) to produce a fresh context snapshot for the resumed role.
6. **Resume execution**: Restart the suspended process (e.g., MT loop) with the new model and fresh context.
7. **Emit completion event**:
   - Success: `FR-EVT-MODEL-002` (swap complete)
   - Failure: `FR-EVT-MODEL-003` (swap failed), including error + rollback notes

**Hard requirements**
- A swap MUST NOT proceed unless `state_hash` matches the persisted state contents.
- A swap MUST NOT exceed the declared resource budgets.
- A swap MUST fail fast on timeout (`timeout_ms`) rather than hanging.
- A swap MUST NEVER drop or mutate Locus state; it may only create new checkpoint artifacts.

##### 4.3.3.4.5 Integration with Micro-Task Executor (Normative)

The MT Executor MUST be able to request model swaps for escalation and role changes, using an execution-policy extension.

```typescript
export interface ExecutionPolicyExtension {
  schema_version: "hsk.exec_policy_ext@0.4";
  kind: "model_swap_policy";

  model_swap_policy: {
    allow_swaps: boolean;
    max_swaps_per_job: number;
    swap_timeout_ms: number;
    fallback_strategy: "abort" | "continue_with_current" | "escalate_to_cloud";
  };
}
```

When `allow_swaps = true`, MT Executor escalation MAY trigger a `ModelSwapRequest` (see Â§2.6.6.8.9 and Â§2.6.6.8.12).


### 4.3.4 Scheduling & Contention


##### 4.3.3.4.5 SwapRequest + escalation rule (Normative) [ADD v02.122]

This spec distinguishes:

- **SwapRequest**: the *workflow-level* requirement (â€œthe system MUST be able to swap models and resume deterministicallyâ€).  
- **ModelSwapRequest**: the concrete runtime command envelope used to execute a swap (defined above).

**SwapRequest requirements (HARD)**

When a SwapRequest is raised (by operator action, policy, or escalation logic), the system MUST:

- preserve state via canonical artifacts (Task Board + WP/MT + gate artifacts) and correlate the swap to Flight Recorder events,
- be able to offload the current frontend model to free resources,
- be able to spin up a larger frontend/orchestrator model on escalation,
- surface any failure as a recoverable failstate with stable code:
  - `CX-MM-003` SwapRequest failed or timed out (see Â§4.3.9.10).

**Escalation trigger (normative)**

If a Work Unit enters FAILSTATE due to model incapability (context limit, tool incapability, repeated failure), the system SHOULD attempt escalation to a higher-ParameterClass model (largest-first), if available, subject to:

- active Work Profile routing policy (see Â§4.3.7.5 and Â§4.3.9.4.2),
- governance constraints (cloud escalation consent artifacts and `allow_cloud_escalation`),
- RuntimeMode/ExecutionMode constraints (see Â§4.3.9.2.3).

The escalation decision MUST be logged (reason + selected model) and the frontend coordinator (even if cloud-based) MUST be notified of the swap decision.


#### 4.3.4.1 The Core Problem

**Only one heavy task can use the GPU efficiently at a time.** Running two things simultaneously doesn't make each run at half speedâ€”it makes both run poorly or crash.

#### 4.3.4.2 Priority Rules

```
Priority Queue (highest to lowest):

  1. Interactive Chat    â†’ User is waiting, <100ms latency matters
  2. Code Generation     â†’ User is waiting, but can tolerate 1-2sec
  3. Image Generation    â†’ User expects to wait (5-30 seconds)
  4. Background Tasks    â†’ Batch processing, can run overnight
```

#### 4.3.4.3 Practical Scheduling Pattern

```python
#  Pseudocode for GPU scheduling
class GPUScheduler:
    def handle_request(self, request):
        if request.priority == "interactive":
            # Pause any batch jobs
            self.pause_background_tasks()
            # Run immediately
            return self.run_now(request)
            
        elif request.priority == "image":
            if self.vram_available() < 10_GB:
                # Not enough VRAM, queue it
                return self.queue(request, 
                    message="Waiting for VRAM...")
            else:
                return self.run_now(request)
                
        else:  # background
            # Only run if GPU is idle
            if self.gpu_is_idle():
                return self.run_now(request)
            else:
                return self.queue(request)
```

---

**Key Takeaways (12.5)**
- âœ“ **Llama 3 13B** is the recommended default general model
- âœ“ **Code Llama 13B** for code tasks, 7B for autocomplete
- âœ“ **SDXL 1.0** via ComfyUI for image generation
- âœ“ Models swap in/out of VRAM based on current task
- âœ“ The 24GB RTX 3090 can handle most scenarios with smart scheduling

â”‚  â”‚ REASONING/      â”‚    â”‚ CREATIVE        â”‚                         â”‚
â”‚  â”‚ PLANNING        â”‚    â”‚ WRITING         â”‚                         â”‚
â”‚  â”‚                 â”‚    â”‚                 â”‚                         â”‚
â”‚  â”‚ Task breakdown  â”‚    â”‚ Fiction         â”‚                         â”‚
â”‚  â”‚ Decision making â”‚    â”‚ Storytelling    â”‚                         â”‚
â”‚  â”‚ Multi-step      â”‚    â”‚ Brainstorming   â”‚                         â”‚
â”‚  â”‚ planning        â”‚    â”‚                 â”‚                         â”‚
â”‚  â”‚                 â”‚    â”‚                 â”‚                         â”‚
â”‚  â”‚ GPT-OSS-20B     â”‚    â”‚ NeuralStar      â”‚                         â”‚
â”‚  â”‚ DeepSeek        â”‚    â”‚ 4x7B MoE        â”‚                         â”‚
â”‚  â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜    â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜                         â”‚
â”‚                                                                      â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
```

#### 4.3.5.2 Expanded Model Recommendations

##### 4.3.5.2.1 General Writing & Reasoning

| Model | Size | VRAM Needed | Strengths | Use For |
|-------|------|-------------|-----------|---------|
| **Llama 3 13B** | 13B | ~14GB (Q4) | Balanced quality/speed | Default text tasks |
| **Mistral 7B** | 7B | ~8GB (Q4) | Fast, efficient | Quick responses |
| **GPT-OSS-20B** | 20B | ~16GB | Strong reasoning | Complex planning |

ðŸ“Œ **Recommendation:** Start with **Llama 3 13B** as the default general model. Use Mistral 7B for fast, simple tasks.

##### 4.3.5.2.2 Code Generation

| Model | Size | VRAM Needed | Strengths | Use For |
|-------|------|-------------|-----------|---------|
| **Code Llama 13B** | 13B | ~14GB (Q4) | Multi-language | Primary code model |
| **Code Llama 7B** | 7B | ~7GB (Q4) | Fast completion | Autocomplete |
| **StarCoder 15B** | 15B | ~15GB | Broad language support | Alternative |

ðŸ“Œ **Recommendation:** **Code Llama 13B** for code generation, 7B variant for real-time autocomplete.

##### 4.3.5.2.3 Image Generation

| Model | Size | VRAM Needed | Strengths | Use For |
|-------|------|-------------|-----------|---------|
| **SDXL 1.0** | ~3B | ~10GB | Best quality | Primary image gen |
| **SD 1.5** | ~1B | ~4GB | Faster, lighter | Quick drafts |

ðŸ“Œ **Recommendation:** **SDXL 1.0** via ComfyUI for quality image generation.

##### 4.3.5.2.4 Creative Writing (Specialized)

| Model | Size | VRAM Needed | Strengths | Use For |
|-------|------|-------------|-----------|---------|
| **NeuralStar AlphaWriter 4x7B** | 24B MoE | ~20GB (Q4) | Fiction-tuned | Stories, creative |

â”€â”€â”€ Nice to Know â”€â”€â”€

> **MoE (Mixture of Experts)** means the model has multiple "expert" sub-models inside. Only some experts activate for each request, making it more efficient than a dense 24B model.

---

#### 4.3.5.3 Memory Budget Planning

â•â•â• CORE CONCEPT â•â•â•

> **You can't run all models at once.** With 24GB VRAM on an RTX 3090, plan which models are loaded when:

```
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚                    VRAM BUDGET (24GB RTX 3090)               â”‚
â”œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¤
â”‚                                                              â”‚
â”‚  SCENARIO A: Text-focused work                              â”‚
â”‚  â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”     â”‚
â”‚  â”‚â–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–‘â–‘â–‘â–‘â–‘â–‘â–‘â–‘â–‘â–‘â–‘â–‘â–‘â–‘â–‘â–‘â–‘â–‘â–‘â–‘â–‘â–‘â–‘â–‘â–‘â–‘â–‘â–‘â–‘â–‘â”‚     â”‚
â”‚  â”‚   Llama 3 13B (14GB)        â”‚     Free (10GB)    â”‚     â”‚
â”‚  â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜     â”‚
â”‚                                                              â”‚
â”‚  SCENARIO B: Image generation                               â”‚
â”‚  â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”     â”‚
â”‚  â”‚â–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–‘â–‘â–‘â–‘â–‘â–‘â–‘â–‘â–‘â–‘â–‘â–‘â–‘â–‘â–‘â–‘â–‘â–‘â–‘â–‘â”‚     â”‚
â”‚  â”‚         SDXL (10GB)         â”‚ Mistral 7B â”‚ Free  â”‚     â”‚
â”‚  â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜     â”‚
â”‚                                                              â”‚
â”‚  SCENARIO C: Code + Chat                                    â”‚
â”‚  â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”     â”‚
â”‚  â”‚â–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–‘â–‘â–‘â–‘â–‘â–‘â–‘â–‘â–‘â–‘â–‘â–‘â–‘â–‘â–‘â–‘â”‚     â”‚
â”‚  â”‚   Code Llama 13B    â”‚   Mistral 7B   â”‚   Free    â”‚     â”‚
â”‚  â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜     â”‚
â”‚                                                              â”‚
â”‚  âš¡ Models swap in/out based on task                        â”‚
â”‚                                                              â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
```

---

### 4.3.6 Local Model Runtimes

**A "runtime" is the software that loads AI models and runs them. Different runtimes have different strengths.**

#### 4.3.6.1 Jargon Glossary [UPDATED v02.190]

| Term | Plain English | Why It Matters for Handshake |
|------|--------------|------------------------------|
| **Ollama** | Easy-to-use model runner | Compatibility reference only; not core Handshake runtime authority |
| **vLLM** | High-performance model server from Berkeley | Reference implementation ideas; not required as an outside daemon |
| **llama.cpp** | Efficient CPU/GPU inference, uses GGUF format | Most flexible for quantized models |
| **ComfyUI workflow engine** | Node-graph implementation for Stable Diffusion workflows | Handshake-managed image-generation engine or explicit compatibility adapter |
| **TGI** | HuggingFace's text generation server | Compatibility/reference only; not core runtime authority |

---

#### 4.3.6.2 Runtime Comparison

| Runtime | Ease of Use | Performance | Flexibility | Best For |
|---------|-------------|-------------|-------------|----------|
| **Handshake ModelRuntime** | â­â­â­â­ | â­â­â­â­ | â­â­â­â­â­ | Core local runtime |
| **ExternalEngineImport adapters** | â­â­â­ | varies | â­â­â­ | Explicit compatibility only |
| **llama.cpp** | â­â­â­ | â­â­â­â­ | â­â­â­â­â­ | Custom setups, edge cases |
| **Handshake-managed ComfyUI engine** | â­â­â­â­ | â­â­â­â­ | â­â­â­â­â­ | Image generation runtime; no operator-launched service prerequisite |

**Key Takeaways (12.7)**
- âœ“ Cloud APIs for planning and complex reasoning (paid but smart)
- âœ“ Local models for execution and bulk work (free)
- âœ“ Automatic fallback when local quality is insufficient
- âœ“ User can override to force local or cloud

â”‚  â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”            â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”        â”‚
â”‚  â”‚ CLOUD (GPT-4)   â”‚            â”‚ LOCAL (Llama)   â”‚        â”‚
â”‚  â”‚                 â”‚            â”‚                 â”‚        â”‚
â”‚  â”‚ Create outline  â”‚â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â–¶â”‚ Write sections  â”‚        â”‚
â”‚  â”‚ and strategy    â”‚            â”‚ based on        â”‚        â”‚
â”‚  â”‚ framework       â”‚            â”‚ outline         â”‚        â”‚
â”‚  â”‚                 â”‚            â”‚                 â”‚        â”‚
â”‚  â”‚ Cost: ~$0.10    â”‚            â”‚ Cost: $0.00     â”‚        â”‚
â”‚  â”‚ (one-time)      â”‚            â”‚ (unlimited)     â”‚        â”‚
â”‚  â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜            â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜        â”‚
â”‚                                                              â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
```

---



### 4.3.7 Work Profile System (Role-Based Model Assignment + Governance Knobs) (Normative) [ADD v02.120]

Work Profiles allow the user (or workspace policy) to define **which models** are used for each runtime role and how autonomous execution is allowed to be (automation level + cloud escalation settings).

This system is the primary UI/config surface for Handshake's â€œmulti-model, local-first, optionally-cloudâ€ direction.

#### 4.3.7.1 WorkProfile Schema (Normative)

Canonical JSON shape:

```typescript
export interface WorkProfile {
  schema_version: "hsk.work_profile@0.5";
  profile_id: string;
  name: string;
  description?: string;

  // Model role assignments
  model_assignments: {
    frontend: ModelAssignment;
    orchestrator: ModelAssignment;
    worker: ModelAssignment;
    validator: ModelAssignment;
  };

  // Governance settings
  governance: {
    automation_level: "FULL_HUMAN" | "HYBRID" | "AUTONOMOUS";
    allow_cloud_escalation: boolean;
    max_cloud_escalations_per_job?: number;
  };

  // Optional override rules
  overrides?: {
    filetype_rules?: Record<string, Partial<WorkProfile["model_assignments"]>>;
    task_type_rules?: Record<string, Partial<WorkProfile["model_assignments"]>>;
  };
}

export interface ModelAssignment {
  primary_model_id: string;
  fallback_model_id?: string;
  local_only: boolean;
  allowed_models?: string[];  // restrict to a whitelist
}
```

**Normative requirements**
- Work Profiles MUST be immutable once used by a job (pin-by-id); new edits create a new `profile_id`.
- A job/session MUST record which `profile_id` was active at execution start.
- Changing the Work Profile MUST emit a Flight Recorder event (`FR-EVT-PROFILE-001`) and MUST NOT retroactively change already-started jobs.

#### 4.3.7.2 UI Control Points (Normative)

Work Profiles MUST be accessible via:

| UI Surface | Control | Function |
|-----------|---------|----------|
| Session Header | Work Profile selector dropdown | Switch profiles (e.g. â€œLocal-onlyâ€, â€œHybridâ€, â€œCloud fallbackâ€) |
| Settings â†’ AI Models | Work Profile editor | Define/edit profiles (models, automation, cloud settings) |
| Job Details Panel | Work Profile display | Show which profile executed the job |
| Capability & Policy Inspector | Work Profile preview | Simulate effects of profile on routing/escalation |

#### 4.3.7.3 Work Profile Selection Examples (Informative)

1. **Local-only profile**
   - `frontend`: small local model (fast)
   - `orchestrator`: medium local model
   - `worker`: Qwen coder local
   - `validator`: local verifier
   - `governance.allow_cloud_escalation = false`

2. **Hybrid profile**
   - Same as local-only, but `validator.fallback_model_id = "gpt-4o"` (cloud) and `allow_cloud_escalation = true`

#### 4.3.7.4 Conformance Tests (Normative)

| Test ID | Requirement |
|--------|-------------|
| T-PROFILE-001 | Work Profile selector persists and emits FR-EVT-PROFILE-001 |
| T-PROFILE-002 | Profile changes do not affect already-running jobs |
| T-PROFILE-003 | Model assignments route correctly (frontend/orchestrator/worker/validator) |
| T-PROFILE-004 | If `local_only = true`, cloud models MUST be rejected |
| T-PROFILE-005 | If cloud escalation disabled, escalation chain MUST stop at HARD_GATE |



#### 4.3.7.5 Work Profile Schema Extensions (Multi-Model + Dynamic Compute) (Normative) [ADD v02.122]

This subsection extends the Work Profile system to support:

- **Multi-model orchestration routing** (largest-first selection, telemetry-informed scoring) (see Â§4.3.9).
- **Per-role compute controls** with an explicit, separate **Approximate** knob, governed by waivers (see Â§4.5 and Waiver Protocol [CX-573F]).

##### 4.3.7.5.1 New/extended types (normative)

```ts
// ADD v02.122
export type ModelBackend = "local" | "cloud";

export type ParameterClass =
  | "P7B"
  | "P13B"
  | "P32B"
  | "P72B"
  | "P110B"
  | "PUnknown";
```

##### 4.3.7.5.2 WorkProfile schema v0.6 (normative)

`hsk.work_profile@0.5` remains valid. This extension introduces `hsk.work_profile@0.6` to add optional routing + compute fields while preserving the existing `model_assignments` contract.

```ts
// ADD v02.122
export interface WorkProfileV06 {
  schema_version: "hsk.work_profile@0.6";
  profile_id: string;
  name: string;
  description?: string;

  // Model role assignments (unchanged shape; extended ModelAssignment below)
  model_assignments: {
    frontend: ModelAssignmentV06;
    orchestrator: ModelAssignmentV06;
    worker: ModelAssignmentV06;
    validator: ModelAssignmentV06;
  };

  // Governance settings (unchanged)
  governance: {
    automation_level: "FULL_HUMAN" | "HYBRID" | "AUTONOMOUS";
    allow_cloud_escalation: boolean;
    max_cloud_escalations_per_job?: number;
  };

  // Optional override rules (unchanged)
  overrides?: {
    filetype_rules?: Record<string, Partial<WorkProfileV06["model_assignments"]>>;
    task_type_rules?: Record<string, Partial<WorkProfileV06["model_assignments"]>>;
  };

  // NEW: multi-model routing policy (optional)
  routing?: {
    selection_policy?: "largest_available" | "explicit";
    // operator-preference ordered candidates (when selection_policy = largest_available)
    candidate_models?: Array<{
      model_id: string;
      backend: ModelBackend;
      parameter_class?: ParameterClass;
    }>;
  };
}

export interface ApproximateControl {
  allowed: boolean;              // default: false (HARD exact)
  waiver_ref?: string;           // REQUIRED when allowed=true
  waiver_expires_at?: string;    // ISO8601Timestamp (optional)
}

export interface ModelAssignmentCompute {
  // Operator-friendly preset; *not* a guarantee (runtime may downgrade)
  speed_preset?: "standard" | "fast_exact" | "fast_approx";

  // Separate knob (+ waiver) for distribution-changing execution
  approximate?: ApproximateControl;

  // Advanced: direct override. (See ExecPolicy in Â§4.5.5.1)
  exec_policy_override?: any;    // ExecPolicy (schema defined in Â§4.5.5.1)

  // Cloud-only knob (if supported by provider/runtime)
  cloud_reasoning_strength?: string | null;
}

export interface ModelAssignmentV06 {
  primary_model_id: string;
  fallback_model_id?: string;
  local_only: boolean;
  allowed_models?: string[];     // restrict to a whitelist

  // NEW
  compute?: ModelAssignmentCompute;
}
```

##### 4.3.7.5.3 Normative rules (routing + compute)

- If `routing.selection_policy = "largest_available"`:
  - selection MUST follow Â§4.3.9.4.2 (ParameterClass â†’ ModelScore â†’ stable tie-break).
  - `routing.candidate_models[]` MUST be treated as an operator-preference-ordered list.
- **Approximate execution is HARD-forbidden unless waived:**
  - Default is `approximate.allowed=false`.
  - If `approximate.allowed=true`, `waiver_ref` is REQUIRED and MUST reference a valid waiver artifact per Waiver Protocol [CX-573F].
  - Without an active waiver, the coordinator MUST downgrade to an exact policy (or route to an exact-capable model/runtime) and MUST record the downgrade in telemetry (Â§4.5.5.2).
- If `compute.cloud_reasoning_strength` is set for a **local** backend, the system MUST NOT treat it as a runtime control and MUST emit `CX-MM-005` (informational) (see Â§4.3.9.10).
- WorkProfile compute settings are **hints**. The runtime MUST either apply them or deterministically downgrade and report the effective policy (see Â§4.5.5.2 and Â§11.5.11).

##### 4.3.7.5.4 RoleExecutionIdentity logging (normative)

Every role output that can affect the workspace MUST be linkable to a RoleExecutionIdentity record (see Â§4.3.9.4.1). At minimum, the Job metadata and Flight Recorder MUST capture:

- `role`
- `model_id`
- `backend`
- `parameter_class` (or `PUnknown`)
- `cloud_reasoning_strength` (if applicable)
- `session_id`
- `model_instance_id` (if multi-model parallelism is active)
- `wp_id` / `mt_id` when operating under Locus work units


### 4.3.8 ComfyUI Workflow Integration

**ComfyUI is a node-based tool for creating images with AI. Instead of just typing a prompt, you can build complex image processing pipelines.**

#### 4.3.8.1 Jargon Glossary

| Term | Plain English | Why It Matters for Handshake |
|------|--------------|------------------------------|
| **ComfyUI** | Visual tool for building AI image generation workflows | Handshake-managed ComfyUI-compatible workflow engine or explicit operator-configured adapter; standalone ComfyUI is not required |
| **Workflow** | A saved pipeline of image processing steps | Can be triggered programmatically |
| **Node** | One step in the pipeline (like "load model" or "apply style") | Building blocks of workflows |
| **Checkpoint** | A saved AI model file | SDXL base, custom fine-tunes |
| **ControlNet** | Guides image generation with poses, edges, etc. | Advanced control over output |

---

#### 4.3.8.2 Why ComfyUI?

â•â•â• CORE CONCEPT â•â•â•

> ComfyUI-compatible workflows are **saved as JSON** and can be **executed through a Handshake-managed engine adapter**. This means:
> 1. Design complex graph pipelines
> 2. Save them as templates
> 3. Submit them through a governed `engine.comfyui` Workflow-Engine job
> 4. Materialize outputs through ArtifactStore + EventLedger, never through localhost or sidecar files as authority

```text
USER IN HANDSHAKE
  -> Workflow Engine job (`engine.comfyui`)
  -> Capability-gated ComfyUI-compatible adapter
  -> Product-managed graph execution
  -> ArtifactStore materialization
  -> EventLedger receipt + manifest + Flight Recorder projection
```

Normative update [UPDATED v02.191]: localhost ComfyUI endpoints, manually launched ComfyUI apps, and engine-local output folders are compatibility inputs only. They are never authority, never a default runtime prerequisite, and never sufficient proof for core Handshake behavior. Core execution is owned by the Workflow Engine and a Handshake-managed adapter; the authoritative output is the ArtifactStore/EventLedger materialization.

---

#### 4.3.8.3 Pre-Built Workflows to Create

| Workflow | Description | Use Case |
|----------|-------------|----------|
| **txt2img_basic** | Simple text to image | Quick generations |
| **txt2img_quality** | High quality with refiner | Final outputs |
| **img2img** | Modify existing image | Variations |
| **inpaint** | Edit parts of image | Touch-ups |
| **upscale** | Increase resolution | Print-ready |

---
**Key Takeaways (12.8)**
- âœ“ **ComfyUI** runs as a separate service, controlled via API
- âœ“ Workflows are JSON files that can be version controlled
- âœ“ Generated images stored with full metadata for reproducibility
- âœ“ Can build progressively complex workflows over time


### 4.3.9 Multi-Model Orchestration & Lifecycle Telemetry (Normative) [ADD v02.122]

**Intent**  
Enable *workflow-level* multi-model orchestration in Handshake (multiple independent model instances, local and/or cloud) **without weakening governance**, and without introducing intra-model distributed inference (no sharding / tensor parallelism).

This section is **project/task-agnostic**. It applies to any Handshake workspace using Work Packets (WP) + Microtasks (MT) under the governed workflow system.

---

#### 4.3.9.1 Scope and non-goals

##### 4.3.9.1.1 In-scope (normative)

- Multiple **independent model instances** executing different WPs/MTs concurrently (workflow/orchestration layer).
- Mixed local + cloud models (project policy may constrain; spec provides contracts).
- Deterministic recovery: any model (or operator) can resume from canonical artifacts.
- Persistent internal collaboration mailbox for role-to-role advice, deliberation, and lifecycle/status signaling (**non-authoritative**).

##### 4.3.9.1.2 Explicit non-goals (HARD)

This section MUST NOT introduce or imply:

- tensor/pipeline/expert parallelism, sharding, or distributed inference of a single model across multiple CPUs/GPUs,
- any requirement that assumes multiple GPUs exist,
- any â€œmulti-deviceâ€ approach where correctness depends on splitting one model across devices.

**Interpretation:** â€œparallelismâ€ means **multiple roles / multiple model instances / multiple WPs/MTs**, not splitting one model.

---

#### 4.3.9.2 Definitions and contracts

##### 4.3.9.2.1 Model identity (normative)

A model MUST be uniquely referencable by a stable identifier:

- **Local:** GGUF filename or local runtime model name
- **Cloud:** provider model name (plus endpoint/account alias if needed)

```yaml
# ADD v02.122
ModelId: string
ModelBackend: enum [local, cloud]

ParameterClass: enum
  - P7B
  - P13B
  - P32B
  - P72B
  - P110B
  - PUnknown
```

##### 4.3.9.2.2 Cloud reasoning strength vs local (normative)

- Cloud models MAY expose a reasoning-strength control. If supported and requested, it MUST be applied and logged.
- Local models do not have a standardized reasoning-strength runtime knob.
  - Local models MAY be tagged with an informational â€œreasoning tierâ€ for logging/UI comparability, but it MUST NOT be treated as a runtime control.

##### 4.3.9.2.3 Runtime modes (normative)

```yaml
# ADD v02.122
RuntimeMode: enum
  - DOCS_ONLY      # no model required; operator edits artifacts; mechanical gates still run
  - AI_ENABLED     # model-backed actions allowed; orchestration may be single or multi-model

ExecutionMode: enum
  - SINGLE_MODEL_MULTI_ROLE
  - MULTI_MODEL_PARALLEL
  - SPLIT_POLICY_AND_PLANNING
```

Notes:
- RuntimeMode is orthogonal to GovernanceMode (GOV_*). Governance enforcement MUST NOT relax under any RuntimeMode.

##### 4.3.9.2.4 Work Unit lock contract (normative)

A **Work Unit** is either a WP or an MT. Locks are mandatory when more than one Work Unit is active.

```yaml
# ADD v02.122
WorkUnitId: string            # wp_id or mt_id (or combined)
LockKey: string               # canonical file path (preferred) OR explicit lock-set id derived from IN_SCOPE_PATHS
LockOwner:
  role: string
  wp_id: string
  mt_id: string | null
  model_instance_id: string | null
```

---

#### 4.3.9.3 Invariants (HARD unless noted)

##### INV-MM-001: Model-optional operation (DOCS_ONLY)

Handshake MUST remain usable with **zero models loaded** in `RuntimeMode=DOCS_ONLY`.

- Operator MUST be able to inspect/edit canonical artifacts directly.
- Mechanical governance features (gates/validators/manifests) MUST remain available with zero models.

##### INV-MM-002: At least one READY model in AI_ENABLED

In `RuntimeMode=AI_ENABLED`, the system MUST maintain:

- `min_ready_models = 1` at all times (local or cloud)
- â€œREADYâ€ means loaded and callable with bounded latency (not merely configured)

If the READY model becomes unavailable:

- model-backed actions MUST softblock with explicit code + reason,
- mechanical features MUST continue to function.

##### INV-MM-003: Strict non-overlap of file scopes under concurrency

When more than one Work Unit is executing:

- Two concurrently executing Work Units MUST NOT modify overlapping file scopes.
- A deterministic lock MUST exist per Work Unit (see Â§4.3.9.2.4).
- On lock conflict, the system MUST BLOCK one Work Unit deterministically with:
  - stable code,
  - explicit reason,
  - required next actions.

##### INV-MM-004: Canonical contracts remain authoritative

Collaboration MUST NOT bypass canonical artifacts.

Any decision that changes:

- scope / requirements,
- acceptance criteria,
- waivers,
- gate state,
- validator verdicts,

MUST be transcribed into authoritative artifacts (WP/MT + gate artifacts + waivers + validation reports).

##### INV-MM-005: Failstates are explicit and recoverable

Failures MUST be surfaced as:

- explicit status in authoritative channel (Task Board + WP/MT status),
- stable CODE + human-readable REASON,
- correlated Flight Recorder event(s),
- recovery hint that points to canonical artifacts (not chat history).

---

#### 4.3.9.4 Work Profile & model selection requirements (normative)

Work Profiles are the primary operator-visible surface for per-role routing and autonomy (see Â§4.3.7). Multi-model orchestration requires additional identity + selection contracts.

##### 4.3.9.4.1 RoleExecutionIdentity (normative)

Every role output that can affect the workspace MUST carry (or be linkable to):

```yaml
# ADD v02.122
RoleExecutionIdentity:
  role: string
  model_id: ModelId | null
  backend: ModelBackend | null
  parameter_class: ParameterClass | null
  cloud_reasoning_strength: string | null
  session_id: string
  model_instance_id: string | null
  wp_id: string | null
  mt_id: string | null
```

##### 4.3.9.4.2 Largest-first selection policy (normative)

WorkProfile role assignments MUST support:

- `selection_policy = "largest_available"` (default option)
- `candidate_models[]` list ordered by operator preference

Largest-first MUST be determined primarily by:

1) ParameterClass (P110B > P72B > P32B > P13B > P7B > PUnknown)  
2) then by ModelScore if available (Â§4.3.9.4.3)  
3) then by stable tie-break order

##### 4.3.9.4.3 Performance telemetry scoring (recommended; schema is normative)

```yaml
# ADD v02.122
ModelPerformanceSnapshot:
  model_id: ModelId
  backend: ModelBackend
  timestamp: string
  tokens_per_second: number | null
  p50_latency_ms: number | null
  p95_latency_ms: number | null
  failure_rate_1h: number | null
  score: number | null
```

If present, ModelScore MUST be derived from logged telemetry (not guesswork).

##### 4.3.9.4.4 Model selector interface (normative; UI is out-of-scope)

The system MUST expose a model-selector mechanism that can:

- list available models,
- show which model is READY (frontend + workers),
- switch READY model(s) deterministically,
- record the selection decision as an auditable event/artifact.

---

#### 4.3.9.5 Orchestration patterns (normative)

##### 4.3.9.5.1 SINGLE_MODEL_MULTI_ROLE

- One READY model instance serves multiple roles via multiplexing.
- Canonical artifacts remain role-separated (WPs/MTs).
- Locks still apply at file-scope level if multiple Work Units are active.

##### 4.3.9.5.2 MULTI_MODEL_PARALLEL

- Multiple READY model instances may execute different WPs/MTs concurrently.
- INV-MM-003 strict non-overlap is mandatory.
- Concurrency limits MUST be explicit:
  - `max_concurrent_instances`
  - `max_concurrent_work_units`

##### 4.3.9.5.3 SPLIT_POLICY_AND_PLANNING

Pattern: separate policy/profile sensitive I/O from planning/validation.

Requirements:

- MUST be expressed as **separate WPs/MTs** (no hidden in-task split).
- Transformations between raw and planned representations MUST be logged and linked.
- Canonical contracts remain binding; Role Mailbox remains advisory.

##### 4.3.9.5.4 DOCS_ONLY

- No models loaded.
- Operator edits canonical artifacts directly.
- Model-backed actions softblock/failstate explicitly, without breaking mechanical tooling.

---

#### 4.3.9.6 Work decomposition for recovery and small contexts (normative)

##### 4.3.9.6.1 Separate WP/MT for robustness (recommended)

Work SHOULD be decomposed into smaller MTs/WPs to preserve recoverability and support small-context models.

##### 4.3.9.6.2 Deterministic resumption (HARD)

Any model (or operator) MUST be able to resume from:

- Task Board status + WP contract + MT definition + telemetry pointers

without relying on chat history.

---

#### 4.3.9.7 Collaboration mailbox taxonomy (normative)

MailboxKind taxonomy is defined in Â§2.6.8.10.0 [ADD v02.122]. The internal collaboration channel (MailboxKind=COLLAB) remains **non-authoritative** and MUST NOT be confused with external MAIL/TASK_INTAKE inboxes.

---

#### 4.3.9.8 Lifecycle telemetry (normative)

##### 4.3.9.8.1 Operator requirement

The operator MUST be able to answer: â€œwhere is the role/model in the WP/MT lifecycle?â€ without verbose context spam.

##### 4.3.9.8.2 Canonical protocol phases (normative)

Use the current protocol blocks as canonical phases:

```yaml
# ADD v02.122
ProtocolPhase: enum
  - BOOTSTRAP
  - SKELETON
  - IMPLEMENTATION
  - HYGIENE
  - EVALUATION
  - HANDOFF
  - BLOCKED
  - FAILSTATE
  - DONE
```

##### 4.3.9.8.3 Standard single-line status marker (normative)

All roles MUST be able to emit a single-line, machine-parseable marker:

```
HSK_STATUS role=<ROLE> wp=<WP_ID> mt=<MT_ID|NONE> phase=<PHASE> state=<RUNNING|WAITING|BLOCKED|DONE|FAILSTATE> model=<MODEL_ID|NONE> backend=<local|cloud|none> pc=<P7B|P13B|P32B|P72B|P110B|PUnknown|NA> rs=<cloud_strength|NA> lock=<OK|CONFLICT> gate=<G0|G1|NONE>:<PASS|BLOCK|PEND> code=<CX-...|NONE>
```

Rules:
- MUST be emitted whenever phase/state changes.
- MUST remain one line (no multi-line dumps).
- MUST be correlated to Flight Recorder events and linkable to WP/MT.
- MUST be shown immediately after gate output when gates run/block.

##### 4.3.9.8.4 Compact universal UI stage set (optional)

To support non-coding surfaces (Docs, Monaco, Terminal, Mail, Calendar), a compact UI stage set MAY be used:

```yaml
UiStage: enum [INTAKE, PLAN, BUILD, CHECK, SHIP, IDLE, BLOCKED, FAILSTATE]
```

Recommended mapping:
- BOOTSTRAP -> INTAKE
- SKELETON -> PLAN
- IMPLEMENTATION -> BUILD
- HYGIENE/EVALUATION -> CHECK
- HANDOFF -> SHIP

If this compact set causes operator confusion, it SHOULD be disabled; ProtocolPhase remains authoritative.

---

#### 4.3.9.9 Swap and escalation (normative)

##### 4.3.9.9.1 SwapRequest (normative event)

The system MUST support a SwapRequest that:

- preserves state (canonical artifacts + Flight Recorder correlation),
- can offload the current frontend model to free resources,
- can spin up a larger frontend model on escalation.

##### 4.3.9.9.2 Escalation rule (normative)

If a Work Unit enters FAILSTATE due to model incapability (context limit, tool incapability, repeated failure):

- the system SHOULD attempt escalation to a higher ParameterClass model (largest-first), if available,
- the decision MUST be logged (reason + selected model),
- the frontend coordinator (even if cloud-based) MUST be notified of the swap decision.

---

#### 4.3.9.10 Softblock / failstate code registry (normative)

##### 4.3.9.10.1 Requirement

All known softblocks/failstates MUST have stable codes over time.

Recommended prefix: `CX-MM-###` (Multi-Model / Orchestration)

Initial reserved set:
- `CX-MM-001` No READY model available in AI_ENABLED
- `CX-MM-002` File-scope lock conflict (overlapping IN_SCOPE_PATHS)
- `CX-MM-003` SwapRequest failed or timed out
- `CX-MM-004` MULTI_MODEL_PARALLEL requested but disabled by policy
- `CX-MM-005` Cloud reasoning strength requested for a local backend (informational)

Codes MUST appear in:
- Task Board / WP / MT status
- HSK_STATUS line (`code=...`)
- Flight Recorder events

---

#### 4.3.9.11 Integration map (informative)

Typical insertion targets / implementation surfaces:

- Runtime / orchestration primitives (RuntimeMode, ExecutionMode, min_ready_models, swap/escalation).
- Work profiles / routing policy (ParameterClass, largest-first selection, telemetry scoring).
- Task Board / WP/MT contracts (file-scope locks, conflict softblocks/failstates, required next actions).
- Mailbox subsystem (MailboxKind taxonomy, persistence, authority boundary).
- Operator consoles / UI telemetry (HSK_STATUS requirements, gate-output adjacency requirement).


## 4.4 Image Generation (Stable Diffusion)

**Why**  
Image generation is a key capability for creative workflows. This section covers how to integrate Stable Diffusion alongside LLM workloads without resource conflicts.

**What**  
Compares SD 1.5 vs SDXL (speed, quality, VRAM), details VRAM requirements and performance, and provides strategies for integrating image generation with LLM workloads.

**Jargon**  
- **Stable Diffusion (SD)**: Open-source AI that generates images from text descriptions.
- **SD 1.5/2.1**: Older versions; smaller, faster, 512Ã—512 output.
- **SDXL**: Newest version; higher quality, 1024Ã—1024 output, heavier.
- **ComfyUI**: Visual workflow tool for Stable Diffusion; more efficient than Automatic1111.
- **Automatic1111**: Popular SD web interface; less efficient but feature-rich.
- **Steps**: Number of denoising iterations; more steps = higher quality but slower.
- **Refiner**: SDXL's second-stage model that adds fine details.

---

### 4.4.1 SD vs SDXL Overview

#### 4.4.1.1 Quick Comparison

```
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¬â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¬â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚                â”‚    SD 1.5/2.1    â”‚         SDXL             â”‚
â”œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¼â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¼â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¤
â”‚ Output Size    â”‚ 512Ã—512          â”‚ 1024Ã—1024                â”‚
â”‚ VRAM Needed    â”‚ 6-8 GB           â”‚ 7-16 GB (varies)         â”‚
â”‚ Speed (3090)   â”‚ ~0.2-0.3s/image  â”‚ ~4-10s/image             â”‚
â”‚ Quality        â”‚ Good             â”‚ Excellent                â”‚
â”‚ Best For       â”‚ Quick previews   â”‚ Final outputs            â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”´â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”´â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
```

---

### 4.4.2 VRAM Requirements & Performance

#### 4.4.2.1 Detailed VRAM Breakdown

```
SD 1.5 (512Ã—512, 25 steps):
  â€¢ VRAM:    ~6-8 GB
  â€¢ Speed:   ~0.2-0.3 seconds per image
  â€¢ Rate:    ~4-5 images/second possible on 3090

SDXL Base (1024Ã—1024, 30 steps):
  â€¢ VRAM:    ~6-14 GB (depends on optimizations)
  â€¢ Speed:   ~4-10 seconds per image
  â€¢ With optimizations (OneDiff + Tiny VAE): 
    - VRAM drops to ~6.9 GB
    - Speed improves to ~4 seconds

SDXL with Refiner:
  â€¢ VRAM:    ~7-16 GB
  â€¢ Speed:   ~6-12 seconds per image
  â€¢ Higher quality details
```

âš¡ **Key Finding:** With optimizations, SDXL can run alongside a 7B LLM (4GB + 7GB = 11GB total).

---

### 4.4.3 Integrating with LLM Workloads

#### 4.4.3.1 The Contention Problem

**Image generation and LLM inference compete for the same GPU.**

```
Scenario: User chatting while generating an image

WRONG approach (simultaneous):
  â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
  â”‚  Mistral-7B (4GB) + SDXL (10GB) = 14GB â”‚
  â”‚  Both running = GPU thrashing          â”‚
  â”‚  Result: Both slow, possible crash     â”‚
  â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜

RIGHT approach (serialized + priority):
  â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
  â”‚  1. Chat request arrives               â”‚
  â”‚  2. Pause/queue image generation       â”‚
  â”‚  3. Process chat (fast, <1 sec)        â”‚
  â”‚  4. Resume image generation            â”‚
  â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
```

#### 4.4.3.2 Recommended Strategy

```
â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
                    DECISION POINT: Image Generation
â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•

RECOMMENDED: Handshake-managed ComfyUI engine + Sequential Processing

Setup:
  â€¢ Launch and supervise the ComfyUI-compatible engine through Handshake
  â€¢ Call it through Handshake-owned in-process, subprocess IPC, HTTP, or gRPC transport
  â€¢ Keep LLM models hot; unload for big image jobs

Priority:
  â€¢ Chat/code requests ALWAYS preempt image generation
  â€¢ Queue images, show progress to user
  â€¢ Run image generation when GPU is otherwise idle

VRAM Management:
  â€¢ For quick SD 1.5: Can run alongside 7B model
  â€¢ For quality SDXL: Unload secondary LLM, keep orchestrator

â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
```

---

**Key Takeaways**  
- SD 1.5: Fast (~0.3s/image), lower VRAM (~6-8GB), good for quick previews.
- SDXL: Higher quality, larger output (1024Ã—1024), but needs ~7-14GB and 4-10 seconds per image.
- Never run heavy image generation and LLM inference simultaneouslyâ€”serialize with priority.
- Use a Handshake-managed ComfyUI-compatible engine; standalone ComfyUI HTTP service is compatibility-only when explicitly configured.
- Chat/code requests always preempt image generation; queue images and show progress.

---
### 4.5 Model Orchestration Policy
- Define hot vs on-demand model loading, GPU/CPU selection, and eviction; log load/unload events with VRAM/CPU and throughput metrics.
- Specify fallback flow (e.g., GPU -> CPU -> smaller model) and token/budget guardrails for local vs any optional cloud usage.
- Expose configuration for model pools and budgets; surface contention (GPU mem, queue depth) via observability dashboards.

### 4.6 Tokenization and Metrics Contract (normative)

For AI-autonomous operation, token counts MUST be accurate to ensure budget enforcement and billing (where applicable). 

1. **No String-Split Approximation:** Implementations MUST NOT use whitespace splitting for token counts in production.
2. **Model-Specific Tokenizers:**
   - **GPT-class:** MUST use `tiktoken` or compatible BPE tokenizer.
   - **Llama/Mistral (Handshake ModelRuntime):** MUST fetch the tokenizer configuration from the local runtime manifest or model artifact metadata and use the correct tokenizer (SentencePiece/Tiktoken).
3. **Vibe Tokenizer (Fallback):** If a model-specific tokenizer is unavailable, the system MUST fallback to a "Vibe Tokenizer" which uses a `char_count / 4.0` heuristic. 
   - **Audit Trail:** Vibe Tokenizer usage MUST emit a `metric.accuracy_warning` to the Flight Recorder.
   - **Sync/Async Bridge:** Because `count_tokens` is synchronous, this emission MUST be decoupled from the execution flow (e.g., via fire-and-forget `tokio::spawn` or a dedicated telemetry channel). It MUST NOT block the tokenization logic.
4. **Consistency Invariant:** Token counts emitted to `JobMetrics` (Â§2.6.6.2.7) MUST match the counts used for retrieval budgeting (Â§2.6.6.7.14).

<a id="5-security-observability"></a>

## 4.5 Layer-wise Inference & Dynamic Compute (Exploratory) [ADD v02.122]

Merged source: **Handshake_Layerwise_Inference_SpecDraft_v0.3.md** (2026-01-22).

**Status:** Draft / exploratory (**not** a Phase 1 feature deliverable).  
**Handshake stance:** Layer-wise inference is *not* a core Handshake feature, but we want **strong foundations** so later phases can explore it safely (hooks + governance + observability).

---

### 4.5.1 Scope and non-goals

#### 4.5.1.1 In scope

- A future-proof **runtime contract** extension (`settings.exec_policy`) to request dynamic compute behavior (layer skipping / early exit / speculative decoding / offload strategies).
- Operator-visible **compute presets** in Work Profiles, per role (standard / fast exact / fast approximate).
- A separate, explicit **approximate** knob that is governed (waiver-required).
- Observability:
  - requested vs effective policy,
  - bounded summary metrics always-on,
  - optional high-volume per-token/per-layer trace artifact via references.

#### 4.5.1.2 Out of scope (Phase 1 / near-term)

- No requirement to implement true layer-wise inference in Phase 1.
- No requirement to support every research approach; Phase 1 ships stable **hooks** and **governed operator controls** only.

---

### 4.5.2 Captured operator decisions

#### 4.5.2.1 Not a core feature

Layer-wise inference is exploratory and MUST NOT weaken Handshakeâ€™s primary objective: deterministic, governed work execution (WP/MT), with auditability and local-first reliability.

---

### 4.5.3 Key definitions

#### 4.5.3.1 Exact vs approximate (Handshake meaning)

- **Exact execution:** speedups intended to preserve the modelâ€™s output distribution (within known floating point / quantization drift bounds).
  - Examples: quantization, caching, speculative decoding (draft + verify), compiled kernels.
- **Approximate execution:** distribution-changing execution.
  - Examples: skipping transformer blocks, early exit, adaptive depth, conditional computation that changes the execution path.

Approximate execution is a governance event and requires an explicit waiver.

---

### 4.5.4 Architectural placement inside Handshake

- This is a **Model Runtime Layer** feature (runtimes implement; orchestration requests).
- It is not a new â€œroleâ€; it modifies how a roleâ€™s assigned model executes.
- It is compatible with multi-model orchestration (Â§4.3.9); in DOCS_ONLY, policies are ignored.
- Operator-first control: planners MUST NOT silently enable approximation.

---

### 4.5.5 Runtime contract extension: `settings.exec_policy`

Patch point: Â§2.5.2.1 Model Runtime Layer Contract (implemented [ADD v02.122]).

The Master Spec defines `settings` as a bag for sampling/config controls (temperature, max_tokens, etc.). v02.122 reserves an **optional** `exec_policy` field inside `settings` for future runtime optimizations.

#### 4.5.5.1 `ExecPolicy` schema (conceptual; forward-compatible)

```ts
// ADD v02.122 (conceptual)
type ExecPolicyExactness = "exact" | "approximate";

type ExecPolicyMode =
  | "standard"          // normal full-depth decode
  | "spec_decode"       // speculative decoding (draft + verify)
  | "early_exit"        // terminate at an intermediate layer
  | "layer_skip"        // skip selected layers (static or dynamic)
  | "mixture_of_depths" // per-token varying depth (router-based)
  | "mem_offload";      // layer/kv/cache offload / paging strategy

interface ExecPolicy {
  policy_id?: string;                  // optional named preset (stable id)
  mode: ExecPolicyMode;
  exactness: ExecPolicyExactness;      // REQUIRED; drives UI + guards

  // Budget envelope (a hint, not a guarantee)
  budgets?: {
    max_total_ms?: number;             // end-to-end latency budget
    max_decode_ms?: number;            // decode budget
    max_layers?: number;               // cap depth for early-exit/skip
    max_vram_mb?: number;              // for offload heuristics
    max_ram_mb?: number;
  };

  // Mode-specific settings
  spec_decode?: {
    draft_model_id?: string;           // optional separate draft model
    draft_top_k?: number;              // draft aggressiveness
    max_draft_tokens?: number;         // per cycle
    verify_strategy?: "classic" | "self_speculate";
  };

  early_exit?: {
    exit_layer?: number | "auto";      // fixed layer or adaptive
    confidence_threshold?: number;     // only if auto
  };

  layer_skip?: {
    schedule?: "fixed" | "adaptive";   // fixed pattern vs token-adaptive
    skip_ratio?: number;              // 0..1 (approx)
    protected_layers?: number[];       // never skip (e.g., embeddings/early)
  };

  mem_offload?: {
    strategy?: "kv_paged" | "layer_paged" | "cpu_offload" | "disk_offload";
    prefetch?: boolean;
  };

  // Trace request (advisory)
  trace?: {
    level: "off" | "summary" | "per_token" | "per_layer" | "per_token_per_layer";
    sample_rate?: number;             // 0..1
    artifact?: {
      enabled: boolean;
      format: "hsk.layerwise_trace@0.1";
    };
  };
}
```

#### 4.5.5.2 Normative behavior

1. If `exec_policy` is present, the runtime MUST either:
   - apply it, or
   - deterministically downgrade to a supported policy and report the **effective** policy (see Â§11.5.11).
2. If `exec_policy.exactness = "approximate"` and there is no active waiver (or governance/profile forbids it), the coordinator MUST downgrade to an exact policy (see Â§4.5.6) or route to an exact-capable model/runtime, and MUST record the decision in telemetry.
3. For `task_type = "tool_call"` or validator-style roles, the coordinator SHOULD default to **exact** policies, even if approximate is allowed elsewhere (operator safety bias).

#### 4.5.5.3 Role as an opaque RoleId string

The runtime contractâ€™s `role` parameter MUST be treated as an opaque RoleId string (not a closed enum), to allow future role packs and compute specialization by role.

(Implemented in Â§2.5.2.1 [ADD v02.122].)

---

### 4.5.6 Operator controls: per-role presets + separate â€œapproximateâ€ knob

#### 4.5.6.1 Core roles: extend Work Profiles (future hook)

Work Profiles already define model assignments per core runtime role. The extension is to attach a compute preset to each `ModelAssignment` (implemented as schema hooks in Â§4.3.7.5):

```ts
interface ApproximateControl {
  allowed: boolean;              // default: false (HARD exact)
  waiver_ref?: string;           // REQUIRED when allowed=true
  waiver_expires_at?: string;    // ISO8601Timestamp (optional)
}

interface ModelAssignmentCompute {
  speed_preset?: "standard" | "fast_exact" | "fast_approx"; // operator-friendly
  approximate?: ApproximateControl;                          // separate knob (+ waiver)
  exec_policy_override?: ExecPolicy;                         // advanced users/dev
}

export interface ModelAssignment {
  primary_model_id: string;
  fallback_model_id?: string;
  local_only: boolean;
  allowed_models?: string[];
  compute?: ModelAssignmentCompute; // NEW
}
```

#### 4.5.6.2 Dynamic roles: inheritance model (future hook)

Dynamic roles can be created (project/task-specific). Recommended inheritance model:

- A dynamic role declares a **runtime-role class** it inherits from (e.g., `worker`) unless it provides an override.
- Optionally, the role can include a `compute` override (same schema as above).
- Resolution order:
  1) Job-level explicit override (if permitted),
  2) Dynamic role override,
  3) Work Profile role default,
  4) Runtime default.

This avoids requiring the Work Profile schema to enumerate unbounded role IDs, while still allowing dynamic roles to specialize.

#### 4.5.6.3 Approximate must never be automatic (HARD)

Approximate execution MUST NOT be enabled implicitly by planners, routing, or runtime heuristics.

It must be:

- explicitly enabled by the operator via Work Profile,
- covered by a waiver (Waiver Protocol [CX-573F]),
- logged as an auditable event (requested vs effective policy).

---

### 4.5.7 Observability: summary + per-token/per-layer trace

#### 4.5.7.1 Always-on summary metrics (Flight Recorder)

Layer-wise inference SHOULD use a separate event family (rather than bloating `llm_inference`) so:

- base LLM telemetry stays stable,
- layer-wise extensions can evolve independently.

Conceptual event (implemented as a schema in Â§11.5.11):

```ts
interface LlmExecPolicyEvent extends FlightRecorderEventBase {
  type: "llm_exec_policy";                 // NEW FR-EVT-LLM-0xx

  trace_id: string;
  model_id: string;

  requested_policy_hash?: string;
  effective_policy_hash?: string;

  mode: ExecPolicyMode;
  exactness: ExecPolicyExactness;

  // Summary metrics (examples; keep bounded)
  exit_layer_histogram?: Record<string, number>; // e.g. {"8": 120, "12": 54}
  mean_exit_layer?: number;

  speculative?: {
    accept_rate?: number;
    mean_draft_tokens?: number;
  };

  offload?: {
    strategy?: string;
    cpu_offload_bytes?: number;
    disk_offload_bytes?: number;
  };

  // Link to high-volume trace (never inline)
  trace_artifact_ref?: string | null;      // artifact handle
  trace_artifact_sha256?: string | null;
}
```

#### 4.5.7.2 Per-token/per-layer trace artifact (high volume)

Flight Recorder pattern is â€œbounded event payloads + references for large dataâ€. Per-token/per-layer traces MUST follow that pattern.

Proposed artifact format: `hsk.layerwise_trace@0.1` (JSONL or CBOR)

**Header (single record):**
- `trace_id`
- `model_id`
- `effective_exec_policy` (canonical JSON)
- `created_at`
- `tokenizer_id` (metadata only; no token IDs)

**Per-token record (one per generated token):**
- `token_index` (0..n-1)
- `t_ms` (monotonic offset)
- `exit_layer` (int) or `layers_used` summary
- `skip_ratio` (0..1) if applicable
- `spec_accept` (bool) if spec decode
- `prefill_ms`, `decode_ms`, `verify_ms` (optional)

**Privacy rule:** do **not** store token IDs. If needed for debugging, store only hashed token text or omit token text entirely (default: omit).

#### 4.5.7.3 Performance rules

- Summary events are always-on.
- High-volume traces SHOULD default to **sampling** or **only-on-approximate** to avoid hot-path overhead, but MUST be available automatically when approximate execution is used (operator expectation).

---

### 4.5.8 â€œApproximate modeâ€ explained (operator-facing semantics)

â€œApproximateâ€ is the explicit label for execution modes that may change outputs compared to full-depth decoding.

Examples:
- **Early exit:** stop at layer N and decode from an intermediate representation (faster, but lower fidelity).
- **Layer skipping:** skip late layers on many steps (faster, but may harm reasoning/factuality).
- **Layer reuse:** reuse hidden states with low-rank corrections (can drift if too aggressive).

Non-example:
- **Distribution-preserving speculative decoding** can be â€œfastâ€ without being approximate, because the full model still verifies/governs the final output distribution.

**Handshake guidance (recommended default):**
- Default to exact policies for:
  - tool calls,
  - validators,
  - workflows requiring deterministic correctness (tests/compiles).
- Allow approximate policies primarily for:
  - conversational drafting,
  - creative ideation,
  - low-stakes summarization.
- Always record the effective policy and whether approximate was active (see Â§11.5.11).

---

### 4.5.9 Practical first experiments: LayerSkip local models (informative)

Based on feasibility research, **LayerSkip** is the most practical â€œdynamic depthâ€ starting point because it is integrated into mainstream frameworks and has pre-trained models available, while alternatives like Mixture-of-Depths remain more research-heavy.

Handshake alignment requirements for experiments:
- Experiments should run **locally** first (local-first + learning loop).
- The runtime must support:
  - stable `generate_text(...)` contract,
  - observability hooks,
  - (eventually) LoRA/adapter workflows.

This likely implies Handshake-owned experiment adapters over open-source engines (Transformers, Candle, llama.cpp, mistral.rs, or product-managed subprocesses). vLLM/TGI/Ollama may be studied or used through explicit ExternalEngineImport compatibility adapters, but Phase 1 core proof MUST remain Handshake-native and must not depend on a third-party model-server daemon.

---

### 4.5.10 Future: Handshake Runtime compatibility constraints (informative)

To avoid rewrites if Handshake later builds a custom runtime (HRT):

- HRT MUST implement the same Model Runtime Layer contract and error semantics.
- HRT MUST accept Work Profile routing decisions (model_id resolution) as input, not hard-coded behavior.
- HRT MUST emit schema-validated Flight Recorder event families (unknown IDs rejected).

---

### 4.5.11 Patch targets status (implemented)

The draft specâ€™s patch targets were:

- Â§2.5.2.1 Model Runtime Layer Contract â†’ reserve `settings.exec_policy` as optional field (implemented [ADD v02.122])
- Â§4.3.7 Work Profiles â†’ attach `compute` presets + separate approximate knob (implemented [ADD v02.122])
- Â§11.5 Flight Recorder â†’ add `llm_exec_policy` event schema + `hsk.layerwise_trace@0.1` artifact rules (implemented [ADD v02.122], see Â§11.5.11)

---

### 4.5.12 Open questions (tracked; optional)

- What is the default â€œfast exactâ€ policy per runtime (spec decode availability varies)?
- Which tasks should forcibly disallow approximate, even if enabled in a Work Profile?
- Should per-token/per-layer trace default to â€œonly when approximateâ€ or â€œdeveloper toggled + sampledâ€?

---

## 4.6 ModelRuntime + LocalModelAdapter (Normative) [ADD v02.186]

**Why**
The Ollama-as-primary architecture retired in v02.186 (Section 4.2.4 rewrite, Section 3.6 invariant) leaves a hole: Handshake still needs to load, generate from, score with, and embed against local models, while exposing hooks the new inference-research lab (Section 4.7) depends on. `ModelRuntime` + `LocalModelAdapter` is the in-process replacement. It is the single primitive every `provider="local"` call from LlmClient (Section 4.2.3, CX-101) dispatches through.

**What**
Defines the `ModelRuntime` Rust trait contract, the two GA adapters (`LlamaCppRuntime`, `CandleRuntime`), per-adapter machine-readable capability declarations, the LlmClient routing rule, and the engine-selection heuristic at model-register time.

---

### 4.6.1 `ModelRuntime` primitive contract

`ModelRuntime` is a Rust trait that every local-model adapter implements. Required methods:

- `load(&mut self, spec: LoadSpec) -> Result<ModelId>` -- load a model into VRAM/RAM under the configured `SandboxAdapter` (Section 3.5); returns the `ModelId` that keys all subsequent calls.
- `unload(&mut self, id: ModelId) -> Result<()>` -- release VRAM/RAM and fire ProcessOwnershipLedger STOP row (Section 5.7).
- `generate(&self, req: GenerateRequest) -> TokenStream` -- streaming token API; `GenerateRequest` carries the `ModelId`, prompt, and sampling options; returns a `TokenStream` consumable by upstream callers without blocking.
- `score(&self, id: ModelId, sequence: Vec<u32>) -> Result<Score>` -- log-likelihood scoring for evaluation lanes.
- `embed(&self, id: ModelId, text: &str) -> Result<Embedding>` -- embedding vector (for adapters that ship an embedding head).
- `capabilities(&self, id: ModelId) -> Result<&ModelCapabilities>` -- see Section 4.6.3.
- `kv_cache(&self, id: ModelId) -> Result<KvCacheHandle>` -- for prefix-cache + KV-quant operations (LlamaCppRuntime).
- `lora_stack(&self, id: ModelId) -> Result<LoraStackHandle>` -- for LoRA hot-swap (both adapters).
- `steering_hooks(&self, id: ModelId) -> Result<SteeringHookHandle>` -- for activation steering / refusal-vector / CAA (CandleRuntime).
- `cancel(&self, token: CancellationToken)` -- cooperative cancellation, must unwind any in-flight `generate`.

Trait surface MUST be engine-agnostic: callers never see `llama_cpp_2::*` or `candle_core::*` types.

The contract is `ModelId`-keyed: a loaded model is addressed by the opaque `ModelId` returned from `load` rather than a separate `ModelHandle`, and per-model accessors (`kv_cache`/`lora_stack`/`steering_hooks`) return `Result` rather than `Option`. This reconciles §4.6.1 with the shipped `ModelRuntime` implementation (KERNEL-004 MT-062). [ADD v02.188]

### 4.6.2 `LocalModelAdapter` invariant

`ModelRuntime` is implemented by per-engine `LocalModelAdapter` types. KERNEL-004 ships **two GA adapters**:

- **`LlamaCppRuntime` (DEFAULT)** -- wraps `llama-cpp-2` Rust crate; covers ~all GGUF transformer models. Path for fast lane, batch lane, code lane. Required for KV-cache + KV-quant + self-speculative decoding (Section 4.7.1g).
- **`CandleRuntime`** -- wraps `candle-core` + `candle-transformers`; required path for hook-requiring techniques (activation steering Section 4.7.1c, refusal vector Section 4.7.1d, CAA Section 4.7.1e) AND for subquadratic architectures (Section 4.7.1h: Mamba2, RWKV v5-v7).

A model is bound to exactly one adapter at register time. Re-binding requires unload + re-register.

### 4.6.3 Per-adapter machine-readable capability declarations

Each adapter declares per-model capability via `ModelCapabilities`:

- `supports_lora`: bool -- hot-swappable LoRA stack supported.
- `supports_kv_prefix_cache`: bool -- radix-tree / prefix-cache KV-sharing.
- `supports_kv_quantization`: enum {none, q4, q8, q4_q8_mix} -- KV-cache quantization levels.
- `supports_activation_steering`: bool -- forward-hook insertion supported.
- `supports_subquadratic`: bool -- non-attention architectures supported (Mamba/RWKV/RetNet).
- `supports_speculative_draft`: bool -- ngram or draft-model speculative decoding.
- `supports_eagle3`: bool -- EAGLE-3 self-speculative decoding (LlamaCppRuntime once llama.cpp PR #18039 merges).

Frontend reads these via Tauri IPC `kernel.model_runtime.capabilities(model_id)` to drive Inference Lab UI (Section 10.14) toggle visibility. Toggles for unsupported capabilities are hidden, not greyed.

### 4.6.4 LlmClient routing rule

LlmClient (Section 4.2.3, CX-101) routes any call where `target.provider == "local"` through `ModelRuntime`. CX-101 HARD_LLM_CLIENT holds: no caller bypasses LlmClient. Adapter dispatch happens inside the runtime layer, not inside the caller.

### 4.6.5 Engine selection at model-register time

At model-register time, operator (or a default heuristic) picks which adapter hosts a given model. Heuristic:

- Transformer GGUF (Llama/Mistral/Qwen/Phi/Gemma/Mixtral families) -> `LlamaCppRuntime`.
- Mamba / Mamba2 / RWKV v5-v7 / RetNet -> `CandleRuntime`.
- Transformer model that the operator wants to use with activation steering / refusal-vector / CAA -> `CandleRuntime` (override, declared in the model-register profile).

The chosen adapter is persisted in the model registry row and surfaced in the ModelRuntime Control Panel (Section 10.13).

**Cross-references:** Section 3.5 SandboxAdapter; Section 3.6 LocalModel boxing invariant; Section 4.2.3 LlmClient; Section 4.2.4 (rewrite); Section 4.7 Inference Research Lab; Section 4.5 layer-wise inference (peer technique); Section 10.13 ModelRuntime Control Panel; Section 10.14 Inference Lab UI; Section 12 primitive registry (Stage 3 will register `ModelRuntime` + `LocalModelAdapter` + `SandboxAdapter`); CX-101 HARD_LLM_CLIENT; WP-KERNEL-004 refinement acceptance criteria AC-MODEL-RUNTIME-TRAIT, AC-LLAMACPP-ADAPTER, AC-CANDLE-ADAPTER, AC-MODEL-CAP-DECL.

---

## 4.7 Inference Research Lab -- Scope and Production Boundary (Normative) [ADD v02.186]

**Why**
The pre-Kernel-V1 spec mentioned "inference research" in scattered places without a normative boundary between what ships in KERNEL-004 and what stays exploratory. The operator-locked decision 2026-05-18 fixes the boundary: **eight production techniques**, plus one deferred technique (Mixture-of-Depths). This section [ADD v02.186] makes the boundary explicit, gates each technique behind a Work Profile knob, and declares the runtime-capability requirement per technique so model registration cannot silently enable an unsupported path.

**What**
Enumerates the eight production techniques, the one spec-deferred technique, the per-technique invariants (runtime-capability requirement, Work Profile knob, observability events, opt-in flag), and the abliteration model-surgery boundary.

---

### 4.7.1 Eight production techniques (KERNEL-004 scope)

Each technique declares its runtime adapter, its Work Profile knob, and its observability event family (FR-EVT-LLM-INFER-*):

- **(a) LoRA hot-swap** -- adapters: `LlamaCppRuntime` + `CandleRuntime` (capability `supports_lora=true`). Work Profile knob: `exec_policy.lora.stack`. Events: FR-EVT-LLM-INFER-LORA-{LOAD,SWAP,UNLOAD}. Use case: per-role / per-Work-Profile persona attachment without full model reload.
- **(b) KV caching (quantization + prefix sharing)** -- adapter: `LlamaCppRuntime` (capability `supports_kv_prefix_cache=true`, `supports_kv_quantization>=q8`). Work Profile knob: `exec_policy.kv.quant` + `exec_policy.kv.prefix_ttl`. Events: FR-EVT-LLM-INFER-KV-{HIT,MISS,EVICT}. Use case: re-using common prompt prefixes (system prompts, ModelManual capsules) across requests.
- **(c) Activation Steering / Representation Engineering** -- adapter: `CandleRuntime` (capability `supports_activation_steering=true`). Work Profile knob: `exec_policy.steering.vectors`. Events: FR-EVT-LLM-INFER-STEER-{APPLY,WITHDRAW}. Use case: forward-hook injection of steering vectors at specific layer indices.
- **(d) Refusal Vector / Refusal Direction Research** -- adapter: `CandleRuntime`. Work Profile knob: `exec_policy.steering.refusal_direction`. Events: FR-EVT-LLM-INFER-REFUSAL-{MEASURE,APPLY}. Use case: per-Work-Profile refusal-direction calibration; research lane only.
- **(e) Contrastive Activation Addition (CAA)** -- adapter: `CandleRuntime`. Work Profile knob: `exec_policy.steering.caa_pairs`. Events: FR-EVT-LLM-INFER-CAA-{DERIVE,APPLY}. Use case: derived steering vector from contrastive prompt pairs.
- **(f) Abliteration** -- **OFFLINE TOOL ONLY**. NEVER inserted into a hot inference path. Output is a derived model artifact, subject to Section 4.8 distillation pipeline content-review (PII scan + license tagging). Work Profile knob: N/A (offline). Events: FR-EVT-LLM-INFER-ABLITERATE-{START,COMPLETE}. See Section 4.7.4.
- **(g) Self-Speculative Decoding** -- adapter: `LlamaCppRuntime` (capability `supports_speculative_draft=true`; `supports_eagle3=true` once llama.cpp PR #18039 merges). Work Profile knob: `exec_policy.speculative.draft_model` + `exec_policy.speculative.mode` (ngram | draft | eagle3). Events: FR-EVT-LLM-INFER-SPEC-{ACCEPT,REJECT}. Use case: ngram/draft speculative GA at v02.186; EAGLE-3 upgrade path tracked.
- **(h) Subquadratic architectures (Mamba2 / RWKV v5-v7)** -- adapter: `CandleRuntime` (capability `supports_subquadratic=true`). Work Profile knob: `exec_policy.subquadratic.model_family`. Events: FR-EVT-LLM-INFER-SUBQ-{STATE_SAVE,STATE_RESTORE}. **Full feature parity required**: LoRA + steering + state-vector persistence + cross-session restore (SSM hidden-state checkpoint to disk, reload on next session resume).

### 4.7.2 One spec-deferred technique: Mixture-of-Depths (MoD)

Mixture-of-Depths (Raposo et al. 2024) is **spec-deferred to Phase 3 roadmap**. KERNEL-004 ships preliminary research only (Section 6.8). Explicit non-actions:

- **NO** new work-packet stub for MoD.
- **NO** new WP created for MoD.
- **NO** runtime adapter capability for MoD in v02.186.

Phase 3 roadmap reflection bullet is added in Stage 3 (out of scope for this enrichment).

### 4.7.3 Per-technique invariants

For each of the eight production techniques:

- **Runtime-capability requirement**: declared in `ModelCapabilities` (Section 4.6.3). Enabling a technique on a model that does not declare the capability is a registration error.
- **Work Profile knob**: extends `settings.exec_policy` schema (declared in Section 4.5.5; v02.186 expands the `exec_policy` schema additively -- see Section 10.14 for the operator UI).
- **Observability events**: `FR-EVT-LLM-INFER-*` family writes typed receipts per CX-130 receipt schema.
- **Opt-in flag**: explicit per-Work-Profile opt-in; defaults are conservative (LoRA off, steering off, abliteration N/A, speculative off, subquadratic only when model family is subquadratic).

### 4.7.4 Abliteration boundary

Abliteration is a **model-surgery class** technique. It mutates model weights to derive a new model artifact. KERNEL-004 invariants:

- Runs offline (never on a hot inference path; never inserted into `ModelRuntime.generate`).
- Output is a new derived model artifact, given its own `ModelId` + on-disk GGUF/safetensors file.
- Derived artifact passes through Section 4.8 content-review pipeline before any Skill Bank entry references it (PII scan + license tagging + provenance).
- ProcessOwnershipLedger row records the abliteration job as a `engine_kind="abliteration_tool"` lifecycle row.

**Cross-references:** Section 4.5 layer-wise inference (sibling technique; same `exec_policy` schema root); Section 4.6 ModelRuntime + LocalModelAdapter; Section 4.8 Distillation Pipeline; Section 6.8 MoD research; Section 10.14 Inference Lab UI; CX-101 HARD_LLM_CLIENT; CX-130 typed receipts; WP-KERNEL-004 refinement acceptance criteria AC-INFER-LAB-8-TECHNIQUES, AC-MOD-DEFERRED, AC-ABLITERATION-OFFLINE.

---

## 4.8 Distillation Pipeline -- Governed-Session-Data Opt-in + Content Review (Normative) [ADD v02.186]

**Why**
The self-improvement loop (KERNEL-004 cluster D) targets validator first-pass PASS rate on a fixed test-packet corpus. Without explicit opt-in, content-review, and Goodhart guards, the loop will (a) leak operator session content into distillation candidates, (b) optimize against the dev set and silently degrade holdout performance, and (c) drift the spec or system prompts through invisible side-channels. This section [ADD v02.186] makes the opt-in default-off, the content-review mandatory, and the editable-surface boundary tight.

**What**
Defines distillation opt-in semantics, content-review pipeline (PII scan + license tagging + dedup), Memory V0+ self-improvement loop target + safeguards, reference patterns adopted/rejected, and the editable-surface boundary for first iteration.

---

### 4.8.1 Opt-in semantics

- **Default**: distillation OFF for every governed session.
- **Opt-in**: per-session, never persistent default. Operator sets `DISTILL_CORPUS=true` at session close (post-hoc opt-in; lets operator review session content before opting in).
- **Scope of opt-in**: a single session's events + artifacts; opt-in does not extend transitively to subsequent sessions.

### 4.8.2 Content-review pipeline

Before any distillation candidate enters the Skill Bank, it MUST pass:

- **PII scan** -- regex + NER detection of names, emails, phone numbers, credentials, machine identifiers. Failures emit `FR-EVT-DISTILL-PII-DETECT` and block promotion.
- **License tagging** -- provenance tagging for any code / text fragment that originated from a third-party source (operator-provided file ingest, web content ingest, model output containing memorized training data flagged by heuristic). Untaggable fragments are quarantined.
- **Dedup** -- content-hash dedup against existing Skill Bank entries; near-dup detection via embedding similarity threshold.

The pipeline **re-uses Section 9 Skill Bank schema**; no new schema is introduced in v02.186.

### 4.8.3 Memory V0+ self-improvement loop (cluster D)

**Target metric**: validator first-pass PASS rate on a fixed ~30-item HBR test-packet corpus.

**HARD safeguards** (operator-binding, 2026-05-18):

- **60/20/20 train/dev/holdout split** on the corpus. Holdout is **encrypted at rest**; loop never sees holdout content during candidate generation.
- **Multi-metric promotion floor**: a candidate may promote only if ALL of (dev PASS rate up, latency p95 not regressed, capsule bytes not regressed, holdout PASS rate not regressed) are satisfied.
- **Goodhart sentinel**: auto-pause if the dev-vs-holdout PASS-rate gap widens monotonically for 3 consecutive iterations.
- **HBR-SWARM-002 loop counter cap**: hard ceiling on iterations per session; loop terminates with `FR-EVT-DISTILL-LOOP-CAP` before unbounded run.
- **PromotionGate operator review**: every promotion requires operator review through the PromotionGate primitive (KERNEL-001).

### 4.8.4 Reference patterns adopted (and rejected)

- **Karpathy autoresearch** -- ADOPTED for loop shape (generate -> evaluate -> score -> propose-next).
- **DSPy MIPROv2** -- ADOPTED for candidate-generation discipline (bayesian-style instruction proposal with prior).
- **SWE-Bench-Pro** -- ADOPTED for holdout split hygiene (encrypted holdout, no leakage path).
- **TextGrad** -- NOT ADOPTED in V0 (gradient-style natural-language optimization; too aggressive for first iteration without spec-shadowing guards).

### 4.8.5 Editable surface for first iteration

**Editable in V0**:

- ModelManual capsule text (Section 10.15) -- the per-model retrieval-augmented context block.
- Retrieval policy parameters (top-k, capsule budget bytes).

**NOT editable in V0** (explicit non-goals):

- Spec text -- shadow-authority risk; spec is operator-edited only.
- System prompts shared across roles -- blast radius too wide.
- LoRA weights -- no training infra in V0; LoRA stacks are operator-authored.

### 4.8.6 Sandbox + ProcessOwnership integration

Distillation candidate-generation jobs run inside a `SandboxAdapter` (Section 3.5) and write `ProcessOwnershipLedger` rows (Section 5.7) with `engine_kind="distillation_candidate_generator"`. Validator scoring runs reuse the KERNEL-003 ValidationRunner.

**Cross-references:** Section 3.5 SandboxAdapter; Section 3.6 boxing invariant; Section 4.7 Inference Research Lab; Section 5.7 ProcessOwnershipLedger; Section 9 Skill Bank (schema reuse); Section 10.15 ModelManual; KERNEL-001 PromotionGate; KERNEL-003 Sandbox + ValidationRunner; WP-1-FEMS-Outcome-Feedback-Loop-v1 (folded into KERNEL-004 cluster D); HBR-SWARM-002 loop cap; CX-130 typed receipts; WP-KERNEL-004 refinement acceptance criteria AC-DISTILL-OPT-IN, AC-DISTILL-CONTENT-REVIEW, AC-DISTILL-LOOP-SAFEGUARDS, AC-DISTILL-EDITABLE-SURFACE.

---
