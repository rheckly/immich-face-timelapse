// Changelog of the fork (rheckly/immich-face-timelapse, branch
// happy-pc) since it split from the original by ArnaudCrl.
//
// Newest version first. Each change: type NEW (feature), CHG (change), FIX
// (bug fix) or LNG (language / translation) and one sentence in German (Swiss spelling, "ss") and English.
// Work that is not pushed yet goes into the entry with `released: false`; on
// a release (push to happy-pc) set `released: true`, the date and the version
// from Cargo.toml, and start a new unreleased entry when the next change comes.

export const CHANGELOG = [
  {
    version: '1.0.2',
    date: '2026-11-14',
    released: true,
    changes: [
      { type: 'NEW', de: `Changelog mit Filtern: Release-Auswahl und Chips für die Art der Änderung (NEW, CHG, FIX, LNG) mit Anzahl.`, en: `Changelog with filters: release selection and chips for the type of change (NEW, CHG, FIX, LNG) with counts.` },
      { type: 'FIX', de: `«Neuer Zeitraffer» setzt Vorspann und Abspann auf die Standardwerte zurück, damit Titel und Bild der vorherigen Person nicht stehen bleiben.`, en: `"New timelapse" sets the intro and outro back to the defaults, so the title and image of the previous person do not stay.` },
      { type: 'CHG', de: `README mit Screenshots der geführten Ansicht: Schritte und Quellen, Qualitätsfilter mit Vorschau, Vorspann, verwendete Fotos und fertiges Video.`, en: `README with screenshots of the guided view: steps and sources, quality filters with preview, intro, used photos and the finished video.` },
      { type: 'CHG', de: `Cargo.toml verweist als Repository auf den Fork rheckly/immich-face-timelapse statt auf das Original.`, en: `Cargo.toml points to the fork rheckly/immich-face-timelapse as repository instead of the original.` },
    ],
  },
  {
    version: '1.0.1',
    date: '2026-11-14',
    released: true,
    changes: [
      { type: 'NEW', de: `Optionale Statistik mit Umami für die App und die öffentlichen Share-Seiten (Umgebungsvariablen UMAMI_URL und UMAMI_WEBSITE_ID); Share-Links werden ohne ihren geheimen Teil gemeldet.`, en: `Optional Umami statistics for the app and the public share pages (environment variables UMAMI_URL and UMAMI_WEBSITE_ID); share links are reported without their secret part.` },
      { type: 'FIX', de: `Veröffentlichung: Die Image-Builds für Branch und Versions-Tag laufen nacheinander, damit das Aufräumen des einen nicht die Teile des anderen löscht.`, en: `Publishing: the image builds for the branch and the version tag run one after another, so the cleanup of one does not delete the parts of the other.` },
      { type: 'CHG', de: `Repository, Docker-Image (ghcr.io/rheckly/immich-face-timelapse) und Container heissen neu «immich-face-timelapse», damit der Fork neben dem Original laufen kann; die Beispiele im README nutzen das eigene Image.`, en: `Repository, Docker image (ghcr.io/rheckly/immich-face-timelapse) and container are now called "immich-face-timelapse" so the fork can run next to the original; the README examples use the fork's own image.` },
      { type: 'CHG', de: `README neu geschrieben: Herkunft und Dank an das Original, Funktionen des Forks, Berechtigungen des Immich-API-Schlüssels, alle Umgebungsvariablen (optionale nur bei Bedarf), Teilen über Port 5001 und Unterschiede zum Original.`, en: `README rewritten: origin and thanks to the original, features of the fork, Immich API key permissions, all environment variables (optional ones only when needed), sharing via port 5001 and differences from the original.` },
      { type: 'NEW', de: `Lizenzhinweis im Panel «Über» (MIT, © Arnaud Cayrol und Änderungen von Roger Heckly) und eigene Copyright-Zeile für die Änderungen in der LICENSE.`, en: `License note in the "About" panel (MIT, © Arnaud Cayrol and modifications by Roger Heckly) and an own copyright line for the modifications in LICENSE.` },
    ],
  },
  {
    version: '1.0.0',
    date: '2026-11-14',
    released: true,
    changes: [
      // Name, look, language
      { type: 'CHG', de: `Umbenennung in «Immich Face-Timelapse» mit neuem Icon und Favicon und einem Hinweis auf das Original-Projekt im README; technische Namen bleiben gleich.`, en: `Renamed to "Immich Face-Timelapse" with a new icon and favicon and a note on the original project in the README; technical names stay the same.` },
      { type: 'CHG', de: `Die geführte Ansicht ist jetzt die Standardansicht; die klassische erscheint nur noch, wenn sie ausdrücklich gewählt wird.`, en: `The guided view is now the default; the classic view only appears when explicitly chosen.` },
      { type: 'LNG', de: `Die geführte Ansicht ist zweisprachig (Deutsch und Englisch) mit Sprachwahl in der Kopfzeile; ohne eigene Wahl gilt die Browsersprache.`, en: `The guided view is bilingual (German and English) with a language switch in the header; without a choice the browser language is used.` },
      { type: 'NEW', de: `Panel «Über» mit Version, Build-Datum, Commit, Herkunft und Status der Immich-Verbindung mit «Erneut versuchen».`, en: `"About" panel with version, build date, commit, origin and Immich connection status with "Retry".` },
      { type: 'NEW', de: `Changelog im Panel «Über»: alle Änderungen seit dem Fork, nach Version gruppiert.`, en: `Changelog in the "About" panel: all changes since the fork, grouped by version.` },
      { type: 'CHG', de: `Versionsanzeige wie bei Immich Mosaik unten links als «Version x · Datum», Commit und Build-Zeit im Tooltip.`, en: `Version display bottom left like Immich Mosaik as "Version x · date", with commit and build time in the tooltip.` },
      { type: 'NEW', de: `Panel «Einstellungen» für globale Werte: Anzahl Versionen pro Person, parallele Abläufe und öffentliche Adresse für geteilte Links.`, en: `"Settings" panel for global values: versions per person, parallel workers and public address for shared links.` },
      { type: 'NEW', de: `Knopf «Neuer Zeitraffer» leert Person, Quellen und Zeitraum; die Einstellungen bleiben (Zurücksetzen unter «Erweitert»). Während ein Zeitraffer läuft, ist er gesperrt.`, en: `"New timelapse" button clears person, sources and period; the settings stay (reset under "Advanced"). It is disabled while a timelapse is running.` },
      { type: 'NEW', de: `Ein laufender Zeitraffer ist immer sichtbar: Oben steht «Läuft: <Name>» mit Fortschritt, «Anzeigen» und «Abbrechen», auch in einem anderen Browser ohne gewählte Person; der Schritt «Zeitraffer erstellen» bleibt dann offen.`, en: `A running timelapse is always visible: on top it shows "Running: <name>" with progress, "Show" and "Cancel", also in another browser without a person picked; the "Create timelapse" step then stays open.` },

      // Photo selection and quality
      { type: 'NEW', de: `Fotos nach Quellen eingrenzen: Alben (Tabelle mit Anzahl und Datum), weitere Personen auf dem Foto und optional «Geteilt mit» aus Immich Smart Albums; es zählt jedes Foto, das zu mindestens einer Quelle passt.`, en: `Limit photos by sources: albums (table with count and date), other people in the photo and optionally "Shared with" from Immich Smart Albums; a photo counts if it matches at least one source.` },
      { type: 'NEW', de: `Eigener Schritt «Zeitraum»: Datumsbereich und Begrenzung der Fotos pro Tag, Woche oder Monat.`, en: `Separate "Period" step: date range and a limit of photos per day, week or month.` },
      { type: 'NEW', de: `Live-Vorschau der Qualitätsfilter an einem Beispielfoto: markiertes Gesicht, entstehendes Bild, Messwerte und Urteil jedes Filters, folgt den Schiebereglern sofort.`, en: `Live preview of the quality filters on a sample photo: marked face, resulting frame, measured values and each filter's verdict, following the sliders instantly.` },
      { type: 'NEW', de: `Fotos vor dem Video auswählen: Nach dem Vorbereiten lassen sich einzelne Fotos per Klick herausnehmen oder wieder aufnehmen; herausgenommene Fotos werden nicht gelöscht.`, en: `Pick photos before the video: after preparation, single photos can be taken out or put back with a click; removed photos are not deleted.` },
      { type: 'CHG', de: `Schritt «Fotos auswählen»: Ein Klick auf die Box öffnet die Auswahl, die Box zeigt danach, wie viele Fotos in den Zeitraffer kommen (z. B. 8 / 12); der Knopf «Video erstellen» steht neu im Schritt «Video erstellen».`, en: `"Pick photos" step: clicking the box opens the selection, which then shows how many photos go into the timelapse (e.g. 8 / 12); the "Create video" button now sits in the "Create video" step.` },
      { type: 'CHG', de: `«Video anschauen» bei früheren Zeitraffern öffnet das Video in einem eigenen Panel rechts von den Details; «Fertig» schliesst es und führt zurück zu den Details.`, en: `"Watch video" on earlier timelapses opens the video in its own panel to the right of the details; "Done" closes it and returns to the details.` },
      { type: 'NEW', de: `Die Details eines früheren Zeitraffers zeigen unter «Verwendete Fotos >» die Fotos in einem Panel rechts; «Fertig» führt zurück zu den Details.`, en: `The details of an earlier timelapse show the used photos under "Used photos >" in a panel to the right; "Done" returns to the details.` },
      { type: 'FIX', de: `«Video erstellen» nach dem Auswählen der Fotos schlug mit «Failed to deserialize query string» fehl.`, en: `"Create video" after picking the photos failed with "Failed to deserialize query string".` },
      { type: 'NEW', de: `Fotos eines fertigen Zeitraffers nachträglich an- oder abwählen und das Video neu erstellen.`, en: `Select or deselect photos of a finished timelapse afterwards and re-create the video.` },
      { type: 'FIX', de: `Albumfotos werden über /search/metadata geladen, weil neuere Immich-Versionen sie nicht mehr in GET /albums/{id} liefern.`, en: `Album photos are loaded via /search/metadata because newer Immich versions no longer list them in GET /albums/{id}.` },

      // Format and look of the video
      { type: 'NEW', de: `Wählbares Seitenverhältnis: Quadrat 1:1, Hochformat 4:5, Querformat 5:4, Breit 16:9 und Senkrecht 9:16; die Grösse gilt für die lange Seite.`, en: `Selectable aspect ratio: square 1:1, portrait 4:5, landscape 5:4, wide 16:9 and vertical 9:16; the size applies to the long edge.` },
      { type: 'CHG', de: `Gesichtsausrichtung und Zuschnitt funktionieren für jedes Seitenverhältnis: Augenposition und Augenabstand sind Anteile des Bildes, mit grafischer Anzeige von Ausrichtung und ergänztem Rand.`, en: `Face alignment and cropping work for every aspect ratio: eye position and eye distance are shares of the frame, with a graphical indicator of alignment and added padding.` },
      { type: 'NEW', de: `Datumseinblendung erweitert: Schriftgrösse, viele Datumsformate (auch mit Monats- und Wochentagsnamen auf Deutsch oder Englisch), TrueType-Schriften, Farbe und Schatten.`, en: `Date overlay extended: font size, many date formats (including month and weekday names in German or English), TrueType fonts, colour and shadow.` },
      { type: 'FIX', de: `Mit der Pixelschrift werden Datumsformate mit Monatsnamen automatisch mit DejaVu Sans Bold gezeichnet, weil die Pixelschrift keine Buchstaben kennt.`, en: `Date formats with month names are drawn with DejaVu Sans Bold automatically, because the pixel font has no letters.` },
      { type: 'CHG', de: `Zusätzliche Schriften im Docker-Image (u. a. DejaVu, Liberation, Open Sans, Lato, Roboto, Montserrat, Bebas Neue, Anton, Archivo Black); weitere Schriften werden auch in config/fonts samt Unterordnern gefunden.`, en: `Additional fonts in the Docker image (including DejaVu, Liberation, Open Sans, Lato, Roboto, Montserrat, Bebas Neue, Anton, Archivo Black); more fonts are also found in config/fonts and its subfolders.` },

      { type: 'NEW', de: `Weitere verspielte Schriften für Datum, Vorspann und Abspann: Lilita One, Fredoka, Chewy und Luckiest Guy.`, en: `More playful fonts for the date, intro and outro: Lilita One, Fredoka, Chewy and Luckiest Guy.` },

      // Intro and outro
      { type: 'NEW', de: `Animierter Vorspann und Abspann: Hintergrund aus Immich-Foto, hochgeladenem Bild oder Farbe; Titel, Untertitel und Text werden nacheinander eingeblendet, mit Abdunkeln, Ein-/Ausblenden und wählbarer Länge.`, en: `Animated intro and outro: background from an Immich photo, an uploaded image or a colour; title, subtitle and text fade in one after another, with dimming, fade in/out and a selectable length.` },
      { type: 'NEW', de: `Schriftart und Farben pro Textblock, optional das Datum des ältesten und jüngsten Fotos, und eine Standbild-Vorschau in der Oberfläche.`, en: `Font and colours per text block, optionally the dates of the oldest and newest photo, and a still preview in the UI.` },
      { type: 'NEW', de: `Bildwähler für Vorspann und Abspann: Foto aus einem Immich-Album, über die Smart-Suche, per Asset-ID oder Immich-Link oder als Upload (JPG, PNG, WebP); frühere Uploads sind wiederverwendbar.`, en: `Image picker for intro and outro: photo from an Immich album, via smart search, by asset ID or Immich link, or as an upload (JPG, PNG, WebP); earlier uploads are reusable.` },
      { type: 'NEW', de: `Bewegung des Hintergrunds wählbar: Standbild, Zoom in die Mitte, Kameraschwenk in vier Richtungen oder zufällig.`, en: `Selectable background motion: still image, zoom into the middle, camera pan in four directions or random.` },
      { type: 'NEW', de: `Bewegung «Automatisch» (Standard): Querformat schwenkt seitlich, Hochformat von oben nach unten, mit sanftem Zoom aufs grösste Gesicht des Fotos.`, en: `"Automatic" motion (default): landscape pans sideways, portrait from top to bottom, with a smooth zoom towards the largest face in the photo.` },

      // Video and music
      { type: 'NEW', de: `Videomodus «Feste Länge»: Die Bildrate ergibt sich aus Anzahl Fotos ÷ gewünschter Länge innerhalb des fps-Bereichs; hat Vorrang vor der Anpassung an die Musik.`, en: `Video mode "Fixed length": the frame rate is number of photos ÷ wanted length within the fps range; takes precedence over fitting to the music.` },
      { type: 'CHG', de: `Vorspann und Abspann zählen zur Videolänge (feste Länge oder Musik), die Fotos laufen entsprechend schneller.`, en: `Intro and outro count towards the video length (fixed length or music), so the photos play correspondingly faster.` },
      { type: 'NEW', de: `Musik direkt aus der Oberfläche hochladen (bis 200 MB pro Datei); die Stücke werden gleich gewählt, bestehende Dateien nicht überschrieben.`, en: `Upload music directly from the UI (up to 200 MB per file); the tracks are selected right away and existing files are not overwritten.` },
      { type: 'CHG', de: `Musikwahl überarbeitet: Suche, nummerierte Reihenfolge der gewählten Stücke, Gesamtlänge und Hinweis auf gewählte, aber fehlende Stücke.`, en: `Music selection reworked: search, numbered order of the selected tracks, total length and a hint for selected but missing tracks.` },
      { type: 'CHG', de: `Detaillierterer Fortschritt: Phasen «Fotos vorbereiten» und «Video erstellen», Bildfortschritt von ffmpeg, verwendete und weggelassene Fotos mit Grund, Laufzeit sowie Dauer und Länge des Videos.`, en: `More detailed progress: phases "prepare photos" and "create video", frame progress from ffmpeg, used and skipped photos with reason, elapsed time, and duration and length of the video.` },

      // Results, history, sharing
      { type: 'CHG', de: `Jeder Durchlauf schreibt in einen eigenen Ordner <Name>_<JJJJ-MM-TT_HHMM>, statt den bisherigen Ordner zu überschreiben.`, en: `Each run writes to its own folder <name>_<YYYY-MM-DD_HHMM> instead of overwriting the previous folder.` },
      { type: 'CHG', de: `Die Ergebnisliste gruppiert die Versionen pro Person (neueste zuerst) und zeigt alte Ordner weiterhin an.`, en: `The output list groups versions per person (newest first) and still shows legacy folders.` },
      { type: 'NEW', de: `Einstellung «Versionen behalten»: Nach einem erfolgreichen Durchlauf werden ältere Versionen pro Person entfernt (0 = unbegrenzt).`, en: `Setting "Keep versions": after a successful run, older versions per person are removed (0 = unlimited).` },
      { type: 'FIX', de: `Leere Ordner fehlgeschlagener Durchläufe werden entfernt.`, en: `Empty folders of failed runs are removed.` },
      { type: 'NEW', de: `Jeder Durchlauf speichert in run.json, woraus er entstand (Person, Quellen, Zeitraum, Einstellungen), ohne Immich-Adresse oder Schlüssel.`, en: `Each run stores in run.json what it was made from (person, sources, period, settings), without Immich address or key.` },
      { type: 'NEW', de: `Detailansicht früherer Zeitraffer mit Video, Erstellungsdaten und «Zeitraffer bearbeiten», das alle Einstellungen übernimmt und eine neue Version erzeugt.`, en: `Detail view of previous timelapses with video, creation details and "Edit timelapse", which takes over all settings and creates a new version.` },
      { type: 'NEW', de: `«Nur Einstellungen übernehmen» in der Detailansicht: übernimmt Filter, Format, Video und Musik eines früheren Zeitraffers für die aktuell gewählte Person.`, en: `"Take over settings only" in the detail view: takes the filters, format, video and music of an earlier timelapse for the person picked now.` },
      { type: 'CHG', de: `«Frühere Zeitraffer» nach Person gruppiert, mit dem Namen aus Immich statt dem Ordnernamen, Datum, Videolänge und den Markierungen «läuft» und «unvollständig».`, en: `"Previous timelapses" grouped by person, with the name from Immich instead of the folder name, date, video length and the marks "running" and "incomplete".` },
      { type: 'NEW', de: `Option «Originale ablegen»: Die verwendeten Fotos aus Immich werden im Unterordner originals gespeichert.`, en: `Option "Keep originals": the Immich photos used are saved in the originals subfolder.` },
      { type: 'NEW', de: `Option «Bearbeitete Bilder behalten»: Ohne Haken werden die ausgerichteten Einzelbilder nach dem Video gelöscht.`, en: `Option "Keep processed images": when unchecked, the aligned frames are deleted after the video is made.` },
      { type: 'NEW', de: `Ergebnis: Video herunterladen und in neuem Tab öffnen.`, en: `Result: download the video and open it in a new tab.` },
      { type: 'NEW', de: `Videos über private Links teilen: Titel, wählbare Gültigkeit, Links verwalten und zurückziehen.`, en: `Share videos via private links: title, selectable validity, manage and revoke links.` },
      { type: 'NEW', de: `Geteilte Links laufen über einen eigenen Port 5001, der nur die Freigabeseite mit Player, Linkvorschau, Video und Vorschaubild ausliefert.`, en: `Shared links are served on a separate port 5001 that only delivers the share page with player, link preview, video and poster image.` },
      { type: 'NEW', de: `Pro Link wählbar: Video herunterladen und bearbeitete Fotos als ZIP herunterladen (beides standardmässig aus).`, en: `Per link: allow downloading the video and the edited photos as ZIP (both off by default).` },

      // Build and operation
      { type: 'CHG', de: `Eigene Versionsnummer ab 1.0.0 aus Cargo.toml, auch im CI-Build angezeigt.`, en: `Own version numbering from 1.0.0 taken from Cargo.toml, also shown in the CI build.` },
      { type: 'CHG', de: `Skript scripts/deploy-nas.sh und Dockerfile-Stufe dev-export für schnelle Test-Builds auf dem NAS.`, en: `Script scripts/deploy-nas.sh and Dockerfile stage dev-export for quick test builds on the NAS.` },
      // Stability and safety
      { type: 'FIX', de: `Während ein Abbruch noch läuft, kann kein zweiter Zeitraffer starten; ein beendeter Lauf löscht nicht mehr den Abbruch-Schalter des nächsten.`, en: `While a cancel is still in progress no second timelapse can start; a finished run no longer clears the cancel switch of the next one.` },
      { type: 'FIX', de: `Die Vorschau prüft die Einstellungen wie ein richtiger Lauf, bevor sie sie übernimmt.`, en: `The preview validates the settings like a real run before using them.` },
      { type: 'FIX', de: `Ordnernamen wie «.» oder nur aus Leerzeichen werden abgelehnt.`, en: `Folder names like "." or only spaces are rejected.` },
      { type: 'NEW', de: `Ändernde Aufrufe und der Fortschritts-Kanal werden nur von der eigenen Seite angenommen (Origin-Prüfung).`, en: `Changing requests and the progress channel are only accepted from the app's own page (origin check).` },
      { type: 'FIX', de: `Eine Anzahl gleichzeitiger Bearbeitungen von 0 bringt den Lauf nicht mehr zum Stillstand.`, en: `A parallel worker count of 0 no longer stalls the run.` },
      { type: 'FIX', de: `ffmpeg schreibt das Video zuerst in eine .part-Datei; ein abgebrochenes Video hinterlässt keine kaputte MP4.`, en: `ffmpeg writes the video to a .part file first; a cancelled video no longer leaves a broken MP4.` },
      { type: 'CHG', de: `Beim Aufräumen alter Versionen werden unvollständige Läufe zuerst entfernt; standardmässig bleiben 5 Versionen pro Person.`, en: `When old versions are pruned, incomplete runs are removed first; by default 5 versions per person are kept.` },
      { type: 'CHG', de: `Das Image auf GHCR trägt zusätzlich die App-Version als Tag (z. B. 1.0.0); eigene Release-Tags heissen ft-v1.x.y, die Upstream-Tags lösen nichts mehr aus.`, en: `The GHCR image is also tagged with the app version (e.g. 1.0.0); own release tags are named ft-v1.x.y, upstream tags no longer trigger anything.` },
    ],
  },
  {
    version: '0.1.0',
    date: '2026-11-12',
    released: true,
    note: { de: 'Erste Version des Forks', en: 'First version of the fork' },
    changes: [
      { type: 'NEW', de: `Neue geführte Ansicht: Prozessschritte links, Panels öffnen sich rechts daneben, auf dem Handy als Vollbild; läuft parallel zur klassischen Ansicht.`, en: `New guided view: process steps on the left, panels open to the right, full screen on mobile; runs alongside the classic view.` },
      { type: 'NEW', de: `Die geführte Ansicht umfasst Person, Alben, Qualitätsfilter (mit Detailpanel pro Filter), Video, Musik, Erweitert, Erstellen, Ergebnis und frühere Zeitraffer.`, en: `The guided view covers person, albums, quality filters (with a detail panel per filter), video, music, advanced, create, result and previous timelapses.` },
      { type: 'NEW', de: `Einstellungen werden automatisch gespeichert und lassen sich auf die Standardwerte zurücksetzen.`, en: `Settings are saved automatically and can be reset to their defaults.` },
      { type: 'NEW', de: `Hell- und Dunkelmodus mit Sonne/Mond-Knopf; ohne eigene Wahl gilt die Systemeinstellung.`, en: `Light and dark mode with a sun/moon button; without a choice the system setting is used.` },
      { type: 'NEW', de: `Hintergrundmusik: ein oder mehrere Stücke aus dem Musikordner wählen; sie werden aneinandergehängt, auf die Videolänge gekürzt und am Ende ausgeblendet.`, en: `Background music: pick one or more tracks from the music folder; they are joined, trimmed to the video length and faded out at the end.` },
      { type: 'NEW', de: `Video an die Musik anpassen: Bildrate = Anzahl Bilder ÷ Musiklänge, begrenzt durch eine minimale und maximale Bildrate.`, en: `Fit video to music: frame rate = number of images ÷ music length, clamped to a minimum and maximum frame rate.` },
      { type: 'NEW', de: `Versionsanzeige: Endpunkt /api/version und Versionsbadge in der Kopfzeile.`, en: `Version display: /api/version endpoint and version badge in the header.` },
      { type: 'CHG', de: `GitHub-Action baut das Image für amd64 und arm64, veröffentlicht es auf ghcr.io und behält nur die letzten 3 Versionen.`, en: `GitHub Action builds the image for amd64 and arm64, publishes it to ghcr.io and keeps only the last 3 versions.` },
      { type: 'FIX', de: `Zwei Clippy-Warnungen neuerer Rust-Versionen behoben, damit der CI-Check durchläuft.`, en: `Fixed two Clippy lints from newer Rust versions so the CI check passes.` },
    ],
  },
];
