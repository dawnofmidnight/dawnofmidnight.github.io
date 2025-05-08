#let title = "dawnofmidnight"
#let author = "dawnofmidnight"
#let description = "dawn's home page on the internet"

#set document(
  title: title,
  author: author,
  description: description,
  date: none,
)

#let meta(name, content) = html.elem("meta", attrs: (name: name, content: content))
#show: it => html.elem("html", attrs: (lang: "en"))[
  #html.elem("head")[
    #html.elem("meta", attrs: (charset: "utf-8"))
    #meta("viewport", "width=device-width, initial-scale=1")

    #html.elem("title", title)
    #meta("description", description)
    #meta("og:title", title)
    #meta("og:description", description)
    #meta("og:type", "website")
    #meta("og:image", "/avatar.png")
    #meta("og:url", "https://dawnofmidnight.github.io")
    #meta("theme-color", "#ffdada")
    
    #html.elem("link", attrs: (rel: "apple-touch-icon", sizes: "180x180", href: "/apple-touch-icon.png"))
    #html.elem("link", attrs: (rel: "icon", type: "image/png", sizes: "32x32", href: "/favicon-32x32.png"))
    #html.elem("link", attrs: (rel: "icon", type: "image/png", sizes: "16x16", href: "/favicon-16x16.png"))
    #html.elem("link", attrs: (rel: "manifest", href: "/site.webmanifest"))

    #html.elem("link", attrs: (rel: "stylesheet", href: "/style.css"))
  ]
  #html.elem("body", it)
]

= dawnofmidnight

hello! i'm dawn (fae/faer), an entity on the internet. this site exists does not have much. if you actually want to find me for whatever reason, you can do so at:

- email: #link("mailto:dawnofmidnight@duck.com")[dawnofmidnight\@duck.com]
- discord: dawnofmidnight
- signal: #link("https://signal.me/#eu/mDHXj_UTvqdaKELLQhSKyfhGkmreZw16C2cPorM4ybY6gk91a8hP8b3bgAJhSqCK")[dawn.67]
- github: #link("https://github.com/dawnofmidnight")[dawnofmidnight]
- codeberg: #link("https://codeberg.org/dawnofmidnight")[dawnofmidnight]
