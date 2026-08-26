# Cómo se crea el repo del bucket

Esta carpeta **no se usa desde acá**: es el contenido inicial de un repo aparte,
`mcorcos/scoop-xtal`, que todavía no existe. Está versionada acá para que crearlo sea
copiar y no volver a escribirlo, igual que pasó con el tap de Homebrew.

```bash
gh repo create mcorcos/scoop-xtal --public \
  --description "Bucket de Scoop para Xtal — LaTeX made easy"

cd $(mktemp -d) && git init -b main
cp -R /ruta/a/xtal/packaging/scoop/bucket/. .
rm LEEME-COMO-CREARLO.md          # esto no va al bucket

# El primer manifiesto, para que `scoop bucket add` sirva desde el minuto cero.
/ruta/a/xtal/packaging/scoop/render-manifest.sh <version> --from-release > bucket/xtal.json

git add -A && git commit -m "El bucket de Xtal"
git remote add origin git@github.com:mcorcos/scoop-xtal.git
git push -u origin main
```

Después se prueba en una máquina con Windows:

```powershell
scoop bucket add xtal https://github.com/mcorcos/scoop-xtal
scoop install xtal
xtal doctor
```

**Hace falta una Release con el zip de Windows.** La v0.3.1 es anterior y no lo tiene: el
script corta con un mensaje que lo dice.
