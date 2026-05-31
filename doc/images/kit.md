# Kit de montage Contelia

## Composants

Avant de commencer, veuillez contrôler que vou disposez bien de tous les
composants de votre boîte à histoire Contelia.

- Eléments imprimés
  - Grande boîte principale (1×)
  - Couvercle de la bôite principale (1×)
  - Boîte à encaster (1×)
  - Couvercle supérieur de la boîte à encaster (1×)
  - Couvercle inférieur de la boîte à encaster (1×)
  - Attache en forme de 8 (1×)
  - Entretoises (4×)
  - Boutons circulaires (2×)
  - Bouton d'alimentation (1×)
- Vitre en plastique (1×)
- Vis nylon (4×)
- Carte Raspberry Pi Zero 2 WH (1×)
- Carte Adafruit 1.3" TFT Bonnet avec son stick en caoutchouc (1×)
- Carte PiSugar 3 avec batterie (1×)
- Haut-parleur USB (1×)
- Câble adaptateur micro-USB / USB (1×)
- Carte micro-SD de 64 GB avec son adaptateur SD (1×)

## Préparation

Avant de commencer le montage à proprement parler, il faut déployer le système
d'exploitation Contelia sur la carte micro-SD. Pour se faire, déballez la carte
et son adaptateur, et en fonction des moyens que vous avez à votre disposition,
introduisez la carte micro-SD dans son adaptateur puis l'adaptateur dans votre
ordinateur, ou utilisez directement un lecteur de carte micro-SD.

Selon votre système d’exploitation, différentes techniques s’offrent à vous pour
déployer l’image sur votre carte micro-SD.

### A. balenaEtcher

Le logiciel [balenaEtcher] est un outil graphique très connu permettant de
facilement déployer une image de ce type depuis chaque plateforme (Linux, macOS
et Windows). À vous de voir si vous souhaitez faire confiance à cet éditeur de
logiciel, tout comme vous me faites confiance pour Contelia OS. Il n’est pas
nécessaire ici que je vous explique comment utiliser ce logiciel, vous devriez
assez vite vous en sortir. Un avantage de l’utiliser par rapport à une ligne de
commande, c’est qu’il y a moins de risques de faire une bêtise en écrivant
l’image sur le mauvais périphérique.

### B. Ligne de commande

Si votre système d’exploitation est Linux ou macOS, je ne recommande pas
l’utilisation de [balenaEtcher] car c’est comme utiliser un canon pour écraser
une mouche. Le déploiement de Contelia OS est très simple et ne nécessite aucune
subtilité. Vous pouvez utiliser (en root comme les vrais) la commande `dd` pour
faire ce travail.

**Linux**

Sous Linux, identifiez l’adresse de votre carte micro-SD avec la commande dmesg.

```
dmesg | grep sd
```

Qui par exemple, pourrait vous retourner ceci :

```
[35574.947414] sd 6:0:0:0: [sde] 249737216 512-byte logical blocks: (128 GB/119 GiB)
[35574.949519] sde: detected capacity change from 0 to 249737216
[35574.981583]  sde: sde1 sde2
```

Ce qui nous indique que la carte est accessible en `/dev/sde`. Faites très
attention avec `dd`. Ne vous mélangez pas les pinceaux avec le `if=` et surtout
le `of=`. Vous allez avoir besoin des droits root et il serait dommage de
détruire un de vos périphériques. Rappelez-vous que `if` signifie Input File, et
`of` signifie Output File.

```
# Veuillez remplacer ici le sdX par le numéro qui correspond à votre carte
dd if=~/carnotzet-os-X.Y.Z-rpi4b.img of=/dev/sdX bs=1M status=progress
```

**macOS**

La bonne nouvelle c’est que sous macOS c’est presque la même chose. Comme avec
Linux, faites attention avec les `if=` et `of=`.

```
diskutil list

# Démontez la carte µSD (remplacer diskX par le bon identifiant), puis flashez
diskutil unmountDisk /dev/diskX
sudo dd if=~/carnotzet-os-X.Y.Z-rpi4b.img of=/dev/diskX bs=1m status=progress
```

### Ejection

Comme avec tous périphérique qui se respecte, veuillez éjecter proprement la
carte micro-SD de votre ordinateur. Il faut être sûr que les données sont
correctement écrites. Dans le cas contraire, une fois que votre boîte à histoire
Contelia sera montée, il risquerait de ne rien s'afficher mais nous y
reviendrons plus loin.

## Montage

A partir d'ici, les manipulations vont devoir se faire avec soin et si possible
sans électrocité statique. Si vous ne disposez pas de bracelet anti-statique,
assurez-vous de ne pas porter de vêtements qui favorisent l'électrocité statique
tel qu'un pull-over ou une jacket, ou un vêtement en cuir.

Dans tous les cas, essayez de manipuler les cartes électronique en touchant le
moins possible les contactes et les composants.

### Préparation de la carte Raspberry Pi

Avant tout, insérez la carte micro-SD dans le Raspberry Pi.

![](IMG_20260501_225443.jpg)

Je vous invite alors à retourner le Raspberry Pi et à inspecter les contactes
tout en haut à droite.

![](IMG_20260501_225607.jpg)

Vous devriez constater que 6 contactes sont légèrement différents où des
soudures ont été refaîtes. En effet, ces 6 contactes vont être appuyés contre la
carte d'alimentation PiSugar qui utilise des Bogo-PINs (se sont des PINs montés
sur ressort et qui vont s'appuyer sur les contactes du Raspberry Pi). Si vous ne
constatez pas de différences entre ces contactes et les autres, il est possible
que les connexions avec la carte d'alimentation ne se fassent pas correctement.

### Préparation de la carte PiSugar 3

La carte PiSugar 3 est directement attachée à sa batterie au Lithium. Un aimant
permet de maintenir en place la batterie. Si vous déconnectez l'aimant, faîtes
attention de ne pas toucher des contactes de la carte avec la surface circulaire
en acier. Le cas échéant, il pourrait se produire un court-circuit destructeur.

![](IMG_20260501_230214.jpg)

Posez la carte de telle manière que la batterie se situe en dessous. Vous
devriez constater que les 4 trous filetés ont un autocollant semi-transparent
orange collé dessus.

![](IMG_20260501_230227.jpg)

Vous devez enlever ces 4 autocollants sinon il ne sera pas possible de visser
les cartes entre elles le temps venu. Pour y parvenir, utilisez (par exemple) la
lame d'un cutter afin de soulever délicatement chaque autocollant.

![](IMG_20260501_230551.jpg)

### Assemblage des cartes Raspberry Pi et Adafruit

Pour commencer, l'assemblage des cartes ne concernera que la carte Raspberry Pi
et la carte Adafruit 1.3" TFT Bonnet. N'enlevez pas encore la protection sur
l'écran de la carte Adafruit et laissez le stick de côté pour le moment.

Connectez délicatement la carte Adafruit avec la carte Raspberry Pi. Pour se
faire, introduisez doucement le connecteur en veillant à ne pas appuyer avec vos
doigts sur l'écran. Il serait dommage de l'abimer avec cette manipulation.
Utilisez les zones libre du PCB comme appuis et prennez votre temps.

![](IMG_20260503_103159.jpg)

Une fois connectés, posé les deux cartes à plat et introduisez les entretoises
entre deux et faîtes glisser une vis nylon à l'intérieur afin de maintenir les
entretoises en place.

![](IMG_20260503_103233.jpg)

Répétez cette manipulation avec les 4 coins.

![](IMG_20260503_103339.jpg)

Vous pouvez dès à présent laisser de côté ce montage car nous allons maintenant
nous intéresser à la carte PiSugar 3.

### Assemblage temporaire et premier démarrage

Avant de se lancer plus loin dans le montage, il est judicieux d'effectuer le
tout premier démarrage de Contelia pour s'assurer que tout fonctionne comme
prévu. Si cette étape échoue, il est inutile d'aller plus loin.

Ici, évitez de toucher au bouton d'alimentation de la PiSugar. Bien qu'il y a
par défaut un systpme de protection pour éviter d'allumer la carte par mégarde,
il vaut mieux éviter ce bouton tant que toutes les cartes ne sont pas
correctement vissées entre elles.

Positionnez la carte Raspberry Pi par dessus la carte PiSugar, tout en alignant
les vis. Les connecteurs de la carte PiSugar doivent s'appuyer proprement contre
les contactes de la carte Raspberry Pi.

**PHOTO**

Serrez les vis en croix et assez fermement (mais pas trop non plus, il ne faut
pas abîmer les pas de vis).

Pour ce premier démarrage, je vous invite à brancher un câble d'alimentation
USB-C sur la carte PiSugar. BIen que la batterie devrait être suffisamment
chargée, il est plus sage de s'assurer que la tension électrique ne chute pas
pendant les étapes "critiques" de la première initialisation du Contelia OS.

**A EXPERIMENTER, MESURE LE TEMPS ET EXPLIQUER**

Débranchez le câble USB-C et dévissez toutes les vis. Pour la suite, il est
nécessaire de monter la PiSugar séparément des deux autres cartes.

### Assemblage de la carte PiSugar 3

Avant tout, prennez la petite boîte et le bouton d'alimentation (en forme
rectangulaire). Introduisez le bouton dans la boîte depuis l'intérieur. Ce
bouton ne peut pas tenir en place tant que la carte PiSugar n'est pas installée.
Si vous souhaitez vous simplifier la vie, utilisez un morceau de papier collant
à l'extérieur afin de garder le bouton en place.

![](IMG_20260503_103443.jpg)

Regardez bien le sens de votre boîte, car vous devez insérer la carte PiSugar à
l'intérieur et depuis le bon côté. La carte PiSugar a uniquement un connecteur
USB-C et c'est elle qui a le bouton d'alimentation. Par rapport à la photo
ci-dessus, la carte PiSugar doit alors être insérée par le haut (car nous tenons
la boîte à l'envers).

![](IMG_20260503_103701.jpg)

Vous devez glisser délicatement la carte PiSugar tout en faisant attention à ce
que le bouton d'alimentation se situe bien devant le bouton que vous avez
positionné précédemment avec du papier collant. Une fois fait, ne retournez pas
la boîte.

Saisissez la boîte en gardant appuyé avec un doigt, la carte PiSugar.

![](IMG_20260503_104039.jpg)

Alors vous ppuvez retourner l'assemblage pour y introduire de l'autre côté, la
paire de carte Raspberry Pi et Adafruit que vous avez préparé tout à l'heure.
Tout en gardant bien en place la carte PiSugar, introduisez les autres cartes.
Vous pouvez écarter légèrement les parois latérales de la boîte pour faciliter
leur introduction.

![](IMG_20260503_104604.jpg)

Faîtes en sorte d'introduire les cartes en les gardant à plat autant que
possible afin que les contactes entre la carte PiSugar et la carte Raspberry Pi
se fassent sans encombre. Une fois que vous avez bien introduit toutes les
cartes, les connecteurs de la carte Raspberry Pi (ainsi que le connecteur USB-C
de la PiSugar) devraient être alignés avec les trous du boîtier.

![](IMG_20260503_104644.jpg)

Tout en gardant bien en place avec une main, le trio de carte (je vous rappel
d'éviter autant que possible de presser sur l'écran de la carte Adafruit),
commencez par visser (en croix) les vis nylons. Vissez bien en croix et
gentiment afin que les cartes restent toujours bien plaquées. Ne serrez pas
comme un sourd car vous risqueriez d'abîmer le filetage des vis. Mais serrez
fermement car il faut garantir les appuis avec les contactes de la carte
Raspberry Pi.

![](IMG_20260503_104733.jpg)

### Fermeture de la boîte à encaster

**Couvercle inférieure**

Vous avez deux couvercles à disposition. Le couvercle plein sert évidemment à
fermer l'ouverture sur la batterie.

![](IMG_20260503_103810.jpg)

Faîtes tenir le couvercle au moins d'un côté court et maintenez comme sur la
photo avec une seule main. Utilisez votre seconde main pour courber (avec un peu
de force) légèrement le couvercle afin de faire rentrer le second clip.

![](IMG_20260503_105016.jpg)

**Couvercle supérieure**

Avant tout, vous devez introduire la vitre en plastique transparent dans les
fentes du couvercle. Cette vitre est destinée à protéger l'écran des rayures.

![](IMG_20260503_111229.jpg)

Poussez la vitre autant que possible. Elle ne doit pas obstruer un bouton et
elle doit complètement recouvrir le trou destiné à l'écran. Il est possible que
votre vitre ne soit pas exactement de la même dimension que sur la photo
ci-dessous.

![](IMG_20260503_111326.jpg)

Maintenant il faut préparer la carte Adafruit avant de refermer la boîte.
Commencez par enfoncer le stick en caoutchouc. N'ayez pas peur de presser
fermement sur le stick car il peut y avoir une certaine résistance.

![](IMG_20260503_111432.jpg)

Une fois le stick en place, enlevez délicatement le film protecteur de l'écran.

![](IMG_20260503_111502.jpg)

Déposez alors les deux boutons poussoir comme indiquez sur la photo. Si vous n'y
arrivez pas car c'est un jeu d'équilibrisme, vous pouvez procéder comme avec le
bouton d'alimentation en utilisant du papier collant en faisant tenir les
boutons directement sur le couvercle.

![](IMG_20260503_111525.jpg)

Fixer le couvercle supérieure de la même manière que pour le couvercle
inférieure.

![](IMG_20260503_111729.jpg)

Votre petite boîte à histoire est prête.

![](IMG_20260503_111802.jpg)

---

[balenaEtcher]: https://etcher.balena.io/
