from fastapi import FastAPI, File, Form, HTTPException, UploadFile

from .extraction import MAX_PDF_BYTES, extract_pdf, probe_pdf

app = FastAPI(title="cronogramando-pdf-service")


@app.get("/health")
def health() -> dict:
    return {"status": "ok", "engines": ["normal", "academic"]}


@app.post("/extract")
async def extract(
    file: UploadFile = File(...),
    connector: str = Form("generic"),
) -> dict:
    data = await file.read()
    if not data:
        raise HTTPException(status_code=400, detail="empty PDF upload")
    if len(data) > MAX_PDF_BYTES:
        raise HTTPException(
            status_code=413,
            detail=f"PDF exceeds the {MAX_PDF_BYTES} byte limit",
        )

    try:
        return extract_pdf(data, connector)
    except ValueError as exc:
        raise HTTPException(status_code=422, detail=str(exc)) from exc
    except RuntimeError as exc:
        raise HTTPException(status_code=503, detail=str(exc)) from exc
    except Exception as exc:  # pragma: no cover - defensive
        raise HTTPException(status_code=500, detail=f"extraction failed: {exc}") from exc

@app.post("/probe")
async def probe(
    file: UploadFile = File(...),
    max_pages: int = Form(2),
    connector: str = Form("generic"),
) -> dict:
    data = await file.read()
    if not data:
        raise HTTPException(status_code=400, detail="empty PDF upload")
    if len(data) > MAX_PDF_BYTES:
        raise HTTPException(
            status_code=413,
            detail=f"PDF exceeds the {MAX_PDF_BYTES} byte limit",
        )

    try:
        return probe_pdf(data, max_pages=max_pages)
    except ValueError as exc:
        raise HTTPException(status_code=422, detail=str(exc)) from exc
    except RuntimeError as exc:
        raise HTTPException(status_code=503, detail=str(exc)) from exc
    except Exception as exc:  # pragma: no cover - defensive
        raise HTTPException(status_code=500, detail=f"probe failed: {exc}") from exc

