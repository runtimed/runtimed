# GraphQL Kernel

Be able to run GraphQL queries directly in Markdown documents and Jupyter Notebooks.

```graphql
%server set https://countries.trevorblades.com/graphql
```

```graphql
query Query {
  country(code: "BR") {
    name
    native
    capital
    emoji
    currency
    languages {
      code
      name
    }
  }
}
```
