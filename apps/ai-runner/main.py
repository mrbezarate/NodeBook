import os
import torch
import gc
from fastapi import FastAPI, HTTPException
from pydantic import BaseModel
from transformers import AutoProcessor, AutoModelForCausalLM, BitsAndBytesConfig
from sentence_transformers import SentenceTransformer

app = FastAPI()

embedder = None

class GenerateRequest(BaseModel):
    model: str
    prompt: str
    stream: bool = False
    max_tokens: int = 1024

class EmbedRequest(BaseModel):
    model: str
    input: str | list[str]

@app.post("/api/generate")
async def generate(req: GenerateRequest):
    model_id = req.model
    if model_id not in ["google/gemma-4-E2B-it", "google/gemma-4-E4B-it"]:
        raise HTTPException(status_code=400, detail="Model not supported")
        
    print(f"Loading {model_id} into VRAM for generation...")
    try:
        quantization_config = BitsAndBytesConfig(
            load_in_4bit=True,
            bnb_4bit_compute_dtype=torch.bfloat16
        )
        
        processor = AutoProcessor.from_pretrained(model_id)
        model = AutoModelForCausalLM.from_pretrained(
            model_id,
            device_map="auto",
            quantization_config=quantization_config
        )
        
        messages = [
            {"role": "user", "content": req.prompt},
        ]
        
        inputs = processor.apply_chat_template(
            messages,
            tokenize=True,
            return_dict=True,
            return_tensors="pt",
            add_generation_prompt=True,
        ).to(model.device)
        
        input_len = inputs["input_ids"].shape[-1]
        
        with torch.no_grad():
            outputs = model.generate(**inputs, max_new_tokens=req.max_tokens)
            
        response = processor.decode(outputs[0][input_len:], skip_special_tokens=True)
        
        # SMART UNLOAD: immediately free VRAM
        del model
        del processor
        del inputs
        del outputs
        gc.collect()
        torch.cuda.empty_cache()
        print(f"Unloaded {model_id} from VRAM.")
        
        return {"response": response}
    except Exception as e:
        # Cleanup in case of error too
        gc.collect()
        torch.cuda.empty_cache()
        print(f"Error during generation: {e}")
        raise HTTPException(status_code=500, detail=str(e))

@app.post("/api/embed")
async def embed(req: EmbedRequest):
    global embedder
    if embedder is None:
        print("Loading embedding model...")
        embedder = SentenceTransformer("all-MiniLM-L6-v2", device="cpu")
        print("Embedding model loaded on CPU!")
        
    texts = req.input if isinstance(req.input, list) else [req.input]
    embeddings = embedder.encode(texts).tolist()
    
    return {"embeddings": embeddings}

if __name__ == "__main__":
    import uvicorn
    uvicorn.run(app, host="0.0.0.0", port=8000)
