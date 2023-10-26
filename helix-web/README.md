# Helix, `wasm32` port

## Building

```sh
wasm-pack build
```

## Init

```sh
cd www/

source /usr/share/nvm/init-nvm.sh

nvm use 16

npm install

# possibly
npm audit fix
```

## Running

```sh
cd www/

source /usr/share/nvm/init-nvm.sh

nvm use 16

npm run start
```

## Testing

(none so far)

```sh
wasm-pack test --headless --firefox
```
