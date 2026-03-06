Eine Pipeline baut dein spezialisiertes Docker-Image, schiebt es in eine Registry (z. B. Docker Hub oder die interne Forgejo/Gitea-Registry), und deine eigentliche Test-Pipeline nutzt dieses Image.

Was du jetzt tun musst:
* Secrets anlegen: Gehe in dein Woodpecker-Dashboard zu deinem Repo und lege unter Settings > Secrets die Werte docker_username und docker_password fest.
* Repo-Name anpassen: Ersetze dein-nutzer/rust-ci-image durch deinen echten Docker-Hub-Namen oder deine private Registry-URL.
