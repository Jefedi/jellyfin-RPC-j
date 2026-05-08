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
| Sur un serveur headless (sans Discord installe) | Stack `docker-compose.headless.yml` (Firefox + arRPC + jellyfin-rpc), cf section dediee plus bas |

> **Mon serveur n'a pas Discord installe : je fais quoi ?**
> Deux options :
> 1. Installer le conteneur sur ta machine de bureau (celle ou Discord tourne)
>    et le laisser allume tant que tu veux la presence active. C'est la
>    solution la plus simple.
> 2. Lancer la **stack headless** (Firefox + arRPC + jellyfin-rpc tout en
>    docker) sur ton serveur. Voir [Mode headless](#mode-headless-serveur-sans-discord) plus bas.

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

---

## Mode headless (serveur sans Discord)

Si ton serveur n'a pas de client Discord installe (cas typique : Debian/Proxmox
sans GUI), tu peux quand meme tout faire tenir dans Docker. La stack
`docker-compose.headless.yml` orchestre 3 conteneurs :

```
┌──────────────────────────────────────────────────────────┐
│                   Ton serveur Debian                      │
│                                                            │
│  ┌──────────┐   socket   ┌────────┐   ws:1337   ┌────────┐│
│  │ jellyfin │ ──IPC───►  │ arRPC  │ ──────────► │firefox ││
│  │   -rpc   │            │ daemon │             │ + ext  ││
│  └────┬─────┘            └────────┘             └───┬────┘│
│       │                                              │     │
│       ▼                                              ▼     │
│  Jellyfin API                              Discord Web    │
└──────────────────────────────────────────────────────────┘
                                                       │
                                                       ▼
                                     Discord (vu par tes amis)
```

- **`arrpc`** mime Discord : il pose un socket `/run/discord-ipc/discord-ipc-0`
  (que `jellyfin-rpc` consomme) et expose un WebSocket sur `:1337`.
- **`firefox`** (image `lscr.io/linuxserver/firefox`) execute Firefox dans le
  conteneur avec une UI web noVNC sur `:3000`. Tu y ouvres Discord Web,
  installes l'extension arRPC une fois, et laisses tourner.
- **`jellyfin-rpc`** est strictement le meme binaire que dans le mode standard.

### 1. Cloner et preparer la config

```bash
git clone -b ax https://github.com/Jefedi/jellyfin-rpc-j.git
cd jellyfin-rpc-j
cp config/main.json.example config/main.json
$EDITOR config/main.json   # url, api_key, username
```

### 2. Variables d'environnement (.env)

```bash
cp .env.example .env
$EDITOR .env
```

Edite au minimum `KASM_USER` et `KASM_PASS` (auth de l'UI Firefox). Choisis
un mot de passe long : c'est l'acces a ton compte Discord.

### 3. Build + run

```bash
docker compose -f docker-compose.headless.yml up -d --build
```

Verifie les 3 conteneurs :

```bash
docker compose -f docker-compose.headless.yml ps
docker compose -f docker-compose.headless.yml logs -f jellyfin-rpc
```

`jellyfin-rpc` doit afficher `Connected!` apres quelques secondes (apres que
arRPC ait pose son socket).

### 4. Configuration humaine UNE SEULE FOIS

Le port Firefox est bind par defaut sur `127.0.0.1:3000` (cf `.env`), donc tu
y accedes via SSH tunnel depuis ton PC :

```bash
ssh -L 3000:127.0.0.1:3000 user@ton-serveur
```

Puis sur ton PC : `http://127.0.0.1:3000` -> connecte toi avec
`KASM_USER`/`KASM_PASS` -> tu vois Firefox dans le navigateur.

Dans ce Firefox :

1. Ouvre https://github.com/OpenAsar/arrpc/releases et telecharge le **xpi** de
   l'extension Firefox arRPC. Glisse le sur la fenetre Firefox pour l'installer.
2. Dans les options de l'extension, mets **`ws://arrpc:1337`** comme cible
   (DNS interne docker : `arrpc` resoud vers le conteneur arRPC). **Ne mets pas
   `localhost`** : Firefox a sa propre pile reseau, localhost dans Firefox
   pointe sur lui-meme, pas sur arRPC.
3. Ouvre https://discord.com/app et logue toi.
4. Laisse cet onglet ouvert. Il restera ouvert tant que le conteneur Firefox
   tourne (volume `firefox-config` persistant).

Lance une lecture sur Jellyfin -> ta presence apparait sur Discord (vu par
tes amis) en ~7 s.

### 5. Mises a jour

```bash
git pull origin ax
docker compose -f docker-compose.headless.yml up -d --build
```

Le volume `firefox-config` est persistant, ta session Discord ne se perd pas
entre les rebuilds.

### Securite

- Le port `1337` (WebSocket arRPC) **n'est pas expose sur l'hote** : Firefox
  l'atteint uniquement via le reseau docker interne (DNS service `arrpc`).
- Le port Firefox est bind sur `127.0.0.1` par defaut.
- Auth Kasm obligatoire pour acceder a Firefox.
- **Si tu changes `FIREFOX_BIND=0.0.0.0` pour exposer Firefox sur le LAN** : un
  attaquant qui force Kasm a un acces direct a ton compte Discord. Mets un mot
  de passe **long et unique**, et idealement, garde 127.0.0.1 + SSH tunnel.

### Depannage headless

**`jellyfin-rpc` boucle sur `failed to connect to IPC socket`**
- arRPC n'a pas (encore) pose son socket. `docker compose logs arrpc` doit
  afficher `bridge ready`.
- Verifie que le volume nomme `discord-ipc` est bien monte sur les deux :
  `docker inspect jellyfin-rpc | grep discord-ipc`.

**L'extension arRPC ne pousse rien dans Discord Web**
- Verifie qu'elle pointe sur `ws://localhost:1337` (et pas `wss://...`).
- Recharge l'onglet `discord.com/app` apres install / config.
- Dans la console DevTools de Firefox, l'extension log les events recus.

**Ne pas se faire ban Discord**
- arRPC se contente d'utiliser les API publiques de l'extension navigateur. Il
  n'y a pas de risque connu, mais c'est non-officiel.
