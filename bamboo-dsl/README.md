(im still deciding if this should actually be used. open an issue to discuss your thoughts!)
A DSL for Bamboo's config files. Below is an example for a CI file.
```
(addStep
    :name "Add python"
    :run "/sbin/apk add python3"
)
```