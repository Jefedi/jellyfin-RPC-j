# syntax=docker/dockerfile:1.6
FROM node:20-alpine

RUN apk add --no-cache tini

WORKDIR /app

COPY package.json ./
RUN npm install --omit=dev --no-audit --no-fund

COPY server.js ./

RUN chown -R node:node /app

USER node

ENV NODE_ENV=production \
    NODE_NO_WARNINGS=1 \
    XDG_RUNTIME_DIR=/run/discord-ipc

EXPOSE 1337

ENTRYPOINT ["/sbin/tini", "--"]
CMD ["node", "server.js"]
