from fastapi import FastAPI

app = FastAPI(title="cronogramando-pdf-service")


@app.get("/health")
def health() -> dict:
    return {"status": "ok", "engines": ["normal", "academic"]}
