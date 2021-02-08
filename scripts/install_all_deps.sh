#!/bin/bash
set -xu

function install_rust() {
	echo 'install rust'
	curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
}

function install_docker() {
	echo 'install docker'
	curl -fsSL https://get.docker.com | bash
	sudo groupadd docker
	sudo usermod -aG docker $USER
	newgrp docker
	sudo systemctl start docker
	sudo systemctl enable docker
}

function install_docker_compose() {
	echo 'install docker compose'
	sudo curl -L "https://github.com/docker/compose/releases/download/1.28.2/docker-compose-$(uname -s)-$(uname -m)" -o /usr/local/bin/docker-compose
	sudo chmod +x /usr/local/bin/docker-compose
}

function install_node() {
	echo 'install node'
	curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.37.2/install.sh | bash
	export NVM_DIR="$HOME/.nvm"
	[ -s "$NVM_DIR/nvm.sh" ] && \. "$NVM_DIR/nvm.sh"  # This loads nvm
	nvm install 'lts/*'
	nvm use 'lts/*'
	npm install --global yarn
}

function install_sys_deps() {
	echo 'install system deps'
	sudo apt install libpq-dev cmake gcc g++ postgresql-client-12
}

function install_all() {
	install_sys_deps
	install_rust
	install_docker
	install_docker_compose
	install_node
}

install_all
