const rust = import('./pkg');

rust
    .then(m => {
        return m.run().then((data) => {
            console.log(data);

            console.log("The latest commit to the wasm-bindgen %s branch is:", data.name);
            console.log("%s, authored by %s <%s>", data.commit.sha, data.commit.commit.author.name, data.commit.commit.author.email);
        })
    })
    .catch(console.error);