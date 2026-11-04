# All targets run inside the devcontainer image so the host needs only Docker.
IMAGE := ulttt-dev
RUN := docker run --rm -v "$(CURDIR):/workspace" -w /workspace $(IMAGE)

.PHONY: image shell
image:
	docker build -t $(IMAGE) .devcontainer

shell:
	docker run --rm -it -v "$(CURDIR):/workspace" -w /workspace $(IMAGE) bash
