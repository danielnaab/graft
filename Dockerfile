FROM python:3.12-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    git curl ca-certificates jq \
 && rm -rf /var/lib/apt/lists/*

RUN pip install --no-cache-dir dvc llm llm-bedrock llm-bedrock-anthropic pyyaml

COPY scripts/entrypoint.sh /usr/local/bin/graft-entrypoint
RUN chmod +x /usr/local/bin/graft-entrypoint

ENV AWS_REGION=us-west-2
WORKDIR /work
ENTRYPOINT ["/usr/local/bin/graft-entrypoint"]
CMD ["rebuild"]
