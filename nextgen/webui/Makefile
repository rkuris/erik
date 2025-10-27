.PHONY: build up test clean

IMAGE := solar-heater-webui-tests
COMPOSE := docker-compose -f docker-compose.yml

build:
	@echo "Building $(IMAGE)..."
	docker build -t $(IMAGE) -f Dockerfile .

up:
	@echo "Starting container (attach to stdout)..."
	$(COMPOSE) up --build --remove-orphans

test:
	@echo "Running tests via docker-compose (HTML report to ./playwright-report)..."
	$(COMPOSE) up --build --abort-on-container-exit --exit-code-from webui-tests

clean:
	@echo "Stopping and removing containers, networks, and volumes..."
	$(COMPOSE) down --volumes --remove-orphans || true
	-@docker rmi $(IMAGE) || true
