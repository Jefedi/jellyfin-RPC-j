# syntax=docker/dockerfile:1.6
FROM node:20-alpine

RUN apk add --no-cache tini su-exec

WORKDIR /app

COPY package.json ./
RUN npm install --omit=dev --no-audit --no-fund

COPY server.js entrypoint.sh ./
RUN chmod +x /app/entrypoint.sh \
    && chown -R node:node /app

# Le container demarre en root pour pouvoir chown le volume IPC, puis
# entrypoint.sh drop a node:node avant d'executer server.js.

ENV NODE_ENV=production \
    NODE_NO_WARNINGS=1 \
    XDG_RUNTIME_DIR=/run/discord-ipc

EXPOSE 6463

ENTRYPOINT ["/sbin/tini", "--", "/app/entrypoint.sh"]
CMD ["node", "server.js"]
