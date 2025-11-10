FROM python:3.11-slim

WORKDIR /workspace

# Install dependencies
RUN pip install --no-cache-dir pyyaml

# Copy transformer script
COPY transform.py /transform.py

# Run transformer
CMD ["python", "/transform.py"]
