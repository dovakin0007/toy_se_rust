
async function search(prompt) {
  const results = document.getElementById("results")
  results.innerHTML = "";
  const response = await fetch("/api/search", {
      method: 'POST',
      headers: {'Content-Type': 'text/plain'},
      body: prompt,
  });
  const json = await response.json();
  console.log(json)

  json.forEach(element => {
    [path, tokens] = element
     let some_li = document.createElement("li")
     some_li.innerHTML = "Location: " + path 
     results.appendChild(some_li)
  });

}

let query = document.getElementById("query");
let currentSearch = Promise.resolve()

query.addEventListener("keypress", (e) => {
  if (e.key == "Enter"){
    currentSearch.then(() => search(query.value))
  }
})