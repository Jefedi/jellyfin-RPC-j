# Jellyfin-RPC en Docker

Cette documentation explique comment faire tourner **Jellyfin-RPC** dans un
conteneur Docker pour qu'il pousse en continu ta presence Discord sans avoir
a relancer le binaire a la main.

---

## Important : ou doit tourner le conteneur ?

Discord Rich Presence utilise un **socket IPC local** (`discord-ipc-0` ...
`discord-ipc-9`). Cela veut dire que `jellyfin-rpc` doit pouvoir parler au
client Discord **via un socket Unix qui se trouve sur la meme machine que
Discord**.

Concretement :

| Ou tourne Discord ? | Ou tu dois lancer le conteneur ? |
|---------------------|----------------------------------|
| Sur ton PC (Linux/macOS/Windows + WSL2) | Sur ce **meme PC** |
| Sur un serveur headless (sans Discord installe) | **Pas possible directement** : il n'y a pas de Discord local pour recevoir l'IPC |

> **Mon serveur n'a pas Discord installe : je fais quoi ?**
> Tu as deux options :
> 1. Installer le conteneur sur ta machine de bureau (celle ou Discord tourne)
>    et le laisser allume tant que tu veux la presence active. C'est la
>    solution la plus simple.
> 2. Utiliser un projet comme [arRPC](https://github.com/OpenAsar/arrpc) ou un
>    bridge IPC reseau. Sortie du scope de ce repo.

Le cote serveur Jellyfin (l'API qui dit ce que tu regardes) peut bien sur etre
n'importe ou, c'est le `url` du fichier de config.

---

## Pre-requis

- Docker 24+ et `docker compose` v2
- Discord installe et **lance** sur la machine qui hebergera le conteneur
- Un compte Jellyfin avec une cle d'API

---

## Mise en route

### 1. Cloner le fork

```bash
git clone https://github.com/jefedi/jellyfin-rpc-j.git
cd jellyfin-rpc-j
```

### 2. Preparer la config

```bash
mkdir -p config
cp example.json config/main.json
$EDITOR config/main.json
```

Renseigne au minimum :
- `jellyfin.url` : l'URL publique ou locale de ton serveur Jellyfin
- `jellyfin.api_key` : une cle d'API Jellyfin (Tableau de bord > Cles API)
- `jellyfin.username` : ton (tes) nom(s) d'utilisateur Jellyfin

Tu peux supprimer les sections optionnelles (`imgur`, `images`, `blacklist`,
etc.) si tu n'en as pas besoin.

### 3. Verifier l'UID

Le conteneur tourne par defaut en `uid:gid` `1000:1000` et monte
`/run/user/1000` (le repertoire ou Discord pose son socket IPC). Verifie que
c'est bien ton UID :

```bash
id -u
```

Si tu n'as pas `1000`, edite `docker-compose.yml` :
- `user: "1000:1000"` -> `user: "<TON_UID>:<TON_GID>"`
- `/run/user/1000:/run/user/1000` -> `/run/user/<TON_UID>:/run/user/<TON_UID>`
- `XDG_RUNTIME_DIR: /run/user/1000` -> `/run/user/<TON_UID>`

### 4. Build + run

```bash
docker compose up -d --build
```

Logs en direct :

```bash
docker compose logs -f
```

Tu devrais voir `Connecting to Discord` puis `Connected!`. Lance une
lecture sur Jellyfin, ta presence Discord doit s'afficher dans les
~7 secondes.

### 5. Arret / mise a jour

```bash
docker compose down              # arrete et supprime le conteneur
docker compose pull              # n/a en build local
docker compose up -d --build     # rebuild apres un git pull
```

---

## Variables et options utiles

Dans `docker-compose.yml` :

| Variable | Defaut | Role |
|----------|--------|------|
| `LOG_LEVEL` | `info` | `trace`, `debug`, `info`, `warn`, `error`, `off` |
| `command: -t N` | `7` | intervalle (sec) entre deux verifications Jellyfin |

Exemple : passer en debug ponctuellement :

```bash
LOG_LEVEL=debug docker compose up
```

---

## Variantes de socket Discord selon l'installation

Selon comment Discord est installe, le socket peut etre a un endroit
different. Adapte le bind volume si besoin :

| Installation | Chemin du socket |
|--------------|------------------|
| Discord standard (deb / tar.gz) | `/run/user/<UID>/discord-ipc-0` |
| Flatpak | `/run/user/<UID>/app/com.discordapp.Discord/discord-ipc-0` |
| Snap | `/run/user/<UID>/snap.discord/discord-ipc-0` |
| `/tmp` (vieux client) | `/tmp/discord-ipc-0` |

Pour Flatpak par exemple :

```yaml
volumes:
  - /run/user/1000/app/com.discordapp.Discord:/run/user/1000
```

---

## Depannage

**`Could not connect to Discord`**
- Verifie que le client Discord est bien lance et logue.
- Verifie l'UID (`id -u`) et le bind volume.
- `ls /run/user/$(id -u)/discord-ipc-*` doit lister au moins un socket.

**Le conteneur boucle sur "Trying to connect"**
- Discord n'a pas pose de socket : redemarre Discord, puis le conteneur.

**Aucune activite ne s'affiche meme apres lecture**
- Verifie ton `username` dans `config/main.json` : il est sensible a la casse.
- Passe `LOG_LEVEL=debug` pour voir les requetes a l'API Jellyfin.

---

## Build sans docker compose

```bash
docker build -t jellyfin-rpc:local .
docker run -d --name jellyfin-rpc \
  --restart unless-stopped \
  --user "$(id -u):$(id -g)" \
  -e XDG_RUNTIME_DIR=/run/user/$(id -u) \
  -v "$PWD/config:/config" \
  -v "/run/user/$(id -u):/run/user/$(id -u)" \
  jellyfin-rpc:local
```
