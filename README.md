# Rup

Rup is a clone of the excellent [pup](https://github.com/EricChiang/pup). It's [jq](https://stedolan.github.io/jq/) but for HTML, so you can filter and extract HTML DOM nodes using the css syntax through the command line.

# Available css filters

- [x] element
- [x] #id
- [x] .class
- [ ] selector + selector
- [ ] selector > selector
- [x] [attribute]
- [x] [attribute="value"]
- [x] [attribute^="value"]
- [ ] [attribute~="value"]
- [x] [attribute$="value"]
- [x] [attribute*="value"]
- [x] :first-child
- [x] :last-child
- [ ] :first-of-type
- [ ] :last-of-type
- [ ] :only-child
- [ ] :only-of-type
- [ ] :contains("text")
- [ ] :nth-child(n)
- [ ] :nth-of-type(n)
- [ ] :nth-last-child(n)
- [ ] :nth-last-of-type(n)
- [ ] :not(selector)
- [ ] :parent-of(selector)
