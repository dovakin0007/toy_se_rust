fetch('api/search', {
    method: "POST", // *GET, POST, PUT, DELETE, etc.
    headers: {
      "Content-Type": "text/plain",
    },

    body:"glsl function linear interpolation", // body data type must match "Content-Type" header
  }).then((response) =>console.log(response));