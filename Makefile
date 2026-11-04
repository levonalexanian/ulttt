# All targets run inside the devcontainer image so the host needs only Docker.
IMAGE := ulttt-dev
RUN := docker run --rm -v "$(CURDIR):/workspace" -w /workspace $(IMAGE)

.PHONY: image shell
image:
	docker build -t $(IMAGE) .devcontainer

shell:
	docker run --rm -it -v "$(CURDIR):/workspace" -w /workspace $(IMAGE) bash

.PHONY: frontend
# Serves the frontend at http://localhost:5173.
frontend:
	docker run --rm -it -p 5173:5173 -v "$(CURDIR):/workspace" -w /workspace/frontend $(IMAGE) \
		sh -c "npm install && npm run dev"
