const conditionsObject = {
  conditions: [],
};

document.addEventListener("DOMContentLoaded", () => {
  /////////// Elements ///////////
  const zipFileInput = document.getElementById("zip-input");
  const folderFileInput = document.getElementById("folder-input");
  const templateFileInput = document.getElementById("template-input");

  const excelList = document.getElementById("excel-list");
  const excelFileInput = document.getElementById("excel-file");
  const submitExcelButton = document.getElementById("submit-excel");
  const search = document.getElementById("search");
  const searchButton = document.getElementById("add-search");
  const templateButton = document.getElementById("download-template");

  let loading = false;
  let mainFileName;
  let formData = new FormData();


function updateTotalCount() {
  let total_count = document.getElementById("total-count");
  total_count.textContent = `Total ${excelList.children.length} files`;
}

  /////////// ______ ///////////
  document.addEventListener("click", (e) => {
    console.log(conditionsObject);
  });

  searchButton.addEventListener("click", function (e) {
    let inputPair = document.createElement("div");
    inputPair.className = "inputPair";
    inputPair.style.display = "flex";
    inputPair.innerHTML = `
            <button class="deleteButton">Delete</button>
            <div class="pairContainer" style="display: flex;">
                <input type="text" placeholder="Data" class="dataInput">
                <input type="text" placeholder="Title" class="titleInput">
                <button class="andButton">And</button>
            </div>
        `;
    document.getElementById("search").appendChild(inputPair);

    // Add a new condition to the formData object
    conditionsObject.conditions.push({
      data: "",
      title: "",
      intersections: [],
    });
  });


templateFileInput.addEventListener('change', (e) => {
  let file = e.target.files[0];
  let reader = new FileReader();

  reader.onload = function(e) {
    var data = e.target.result;
    var workbook = XLSX.read(data, {
      type: "binary"
    });

    var first_sheet = workbook.Sheets[workbook.SheetNames[0]];

    const raw_data = XLSX.utils.sheet_to_json(first_sheet, {header: 1});
    console.log(raw_data);

    // Validate the file format
    const expectedHeaders = ['DATA', 'TITLE'];
    const actualHeaders = raw_data[0].map(header => header.trim());

    if (raw_data.length < 2 || !expectedHeaders.every(header => actualHeaders.includes(header))) {
      // Throw an error if the file doesn't match the expected format
      alert("Invalid file format. The template should have at least one row with 'Title' and 'Data' columns.");
      throw new Error("Invalid file format");
    }

    for (let i = 1; i < raw_data.length; i++) {
      let data = raw_data[i][0];
      let title = raw_data[i][1];
      let intersections = [];

      for (let j = 2; j < raw_data[i].length; j += 2) {
        let intersectionData = raw_data[i][j];
        let intersectionTitle = raw_data[i][j + 1];

        intersections.push({
          data: intersectionData,
          title: intersectionTitle,
          intersections: []
        });
      }

      conditionsObject.conditions.push({
        title: title,
        data: data,
        intersections: intersections
      });
    }

    console.log(conditionsObject);

    // Build DOM input pairs based on the conditionsObject
    for (let condition of conditionsObject.conditions) {
      let inputPair = document.createElement("div");
      inputPair.className = "inputPair";
      inputPair.style.display = "flex";
      inputPair.innerHTML = `
        <button class="deleteButton">Delete</button>
        <div class="pairContainer" style="display: flex;">
          <input type="text" placeholder="Data" class="dataInput" value="${condition.data == undefined ? "" : condition.data}">
          <input type="text" placeholder="Title" class="titleInput" value="${condition.title == undefined ? "" : condition.title}">
          <button class="andButton">And</button>
        </div>
      `;
      document.getElementById("search").appendChild(inputPair);

      for (let intersection of condition.intersections) {
        let intersectionPair = document.createElement("div");
        intersectionPair.className = "pairContainer";
        intersectionPair.style.display = "flex";
        intersectionPair.innerHTML = `
          <input type="text" placeholder="Data" class="intersectionDataInput" value="${intersection.data == undefined ? "" : intersection.data}">
          <input type="text" placeholder="Title" class="intersectionTitleInput" value="${condition.title == undefined ? "" : condition.title}">
          <button class="andButton">And</button>
        `;
        inputPair.querySelector(".pairContainer").appendChild(intersectionPair);
      }
    }
  };

  reader.onerror = function (event) {
    // Handle FileReader errors
    alert("Error reading the file.");
    throw new Error("FileReader error");
  };

  reader.readAsBinaryString(file);
});


 document.addEventListener("click", function (e) {
    if (e.target && e.target.classList.contains("andButton")) {
      e.preventDefault();

      let inputPair = document.createElement("div");
      inputPair.className = "pairContainer";
      inputPair.style.display = "flex";
      inputPair.innerHTML = `
                <input type="text" placeholder="Data" class="intersectionDataInput">
                <input type="text" placeholder="Title" class="intersectionTitleInput">
                <button class="andButton">And</button>
            `;
      e.target.parentNode.after(inputPair);
      e.target.remove();


      let conditionIndex = Array.from(
        inputPair.parentNode.parentNode.children
      ).indexOf(inputPair.parentNode);


      // Add a new intersection to the last condition in the formData object
      conditionsObject.conditions[
        conditionIndex
      ].intersections.push({
        data: "",
        title: "",
        intersections: [],
      });
    }

    if (e.target && e.target.classList.contains("deleteButton")) {
      e.preventDefault();

      // Store the parent and grandparent of the delete button in variables
      let parent = e.target.parentNode;
      let grandparent = parent.parentNode;

      // Remove the corresponding condition from the formData object
      let conditionIndex = Array.from(grandparent.children).indexOf(parent);
      conditionsObject.conditions.splice(conditionIndex, 1);

      // Remove the delete button's parent from the DOM
      parent.remove();
    }
  });

  document.addEventListener("input", function (e) {
    if (
      e.target &&
      e.target.parentNode &&
      e.target.parentNode.parentNode &&
      (e.target.classList.contains("titleInput") ||
        e.target.classList.contains("dataInput"))
    ) {
      let inputPair = e.target.parentNode;
      let conditionIndex = Array.from(
        inputPair.parentNode.parentNode.children
      ).indexOf(inputPair.parentNode);

      console.log("Condition index: " + conditionIndex);

      // Update the title or data of a condition
      conditionsObject.conditions[conditionIndex][
        e.target.classList.contains("titleInput") ? "title" : "data"
      ] = e.target.value;
    } else if (
      e.target &&
      e.target.parentNode &&
      e.target.parentNode.parentNode &&
      (e.target.classList.contains("intersectionTitleInput") ||
        e.target.classList.contains("intersectionDataInput"))
    ) {
      let inputPair = e.target.parentNode;
      let conditionIndex = Array.from(
        inputPair.parentNode.parentNode.children
      ).indexOf(inputPair.parentNode);
      let intersectionIndex =
        Array.from(inputPair.parentNode.children).indexOf(inputPair) - 2;

      console.log("Condition index: " + conditionIndex);
      console.log("Intersection index: " + intersectionIndex);

      // Update the title or data of an intersection
      conditionsObject.conditions[conditionIndex].intersections[
        intersectionIndex
      ][
        e.target.classList.contains("intersectionTitleInput") ? "title" : "data"
      ] = e.target.value;
    }
  });

  folderFileInput.addEventListener('change', async(e) => {
        const files = e.target.files;

        for (file of files) {
            formData.append("excel-file[]", file);
            const div = document.createElement("div");
            const li = document.createElement("li");
            const a = document.createElement("a");
            const getFileButton = document.createElement("button");
            getFileButton.textContent = "Get File";
            getFileButton.addEventListener("click", (e) => {
              e.preventDefault();
              const url = URL.createObjectURL(blob);
              const a = document.createElement("a");
              a.href = url;
              a.download = file.name;
              a.click();
            });

            li.textContent = file.name;
            excelList.appendChild(li);

            const button = document.createElement("button");
            button.textContent = "X";
            button.addEventListener("click", () => {
              let li = e.target.closest("li");
              let nodes = Array.from(li.closest("ul").children);
              let index = nodes.indexOf(li);

              console.log("Index: " + index);

              let values = formData.getAll("excel-file[]");
              values.splice(index, 1);
              formData.delete("excel-file[]");

              values.forEach((value, _) => {
                formData.append("excel-file[]", value);
              });

              li.remove();
              console.log(formData);
            });

            div.appendChild(button);
            div.appendChild(getFileButton);

            li.appendChild(div);
        }
  });

  zipFileInput.addEventListener("change", async (e) => {
    const zipReader = new zip.ZipReader(new zip.BlobReader(e.target.files[0]));
    const entries = await zipReader.getEntries();
    entries.forEach(async (entry) => {
      let id = makeid(10);
      let writer;
      let mime;

      if (entry.filename.endsWith(".xlsx")) {
        writer = new zip.BlobWriter(
          "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
        );
        mime =
          "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet";
      } else if (entry.filename.endsWith(".xls")) {
        writer = new zip.BlobWriter("application/vnd.ms-excel");
        mime = "application/vnd.ms-excel";
      }

      let blob = new Blob([await entry.getData(writer)], {
        type: mime,
      });
      let file = new File([blob], entry.filename, {
        type: mime,
      });

      formData.append("excel-file[]", file);

      let rawDate = await entry.rawLastModDate;

      const date = getDate(rawDate, true);

      const div = document.createElement("div");

      const li = document.createElement("li");
      const a = document.createElement("a");
      const getFileButton = document.createElement("button");

      getFileButton.textContent = "Get File";
      getFileButton.addEventListener("click", (e) => {
        e.preventDefault();

        const url = URL.createObjectURL(blob);
        const a = document.createElement("a");
        a.href = url;
        a.download = file.name;
        a.click();
      });

      li.textContent = file.name;
      excelList.appendChild(li);

      const button = document.createElement("button");
      button.textContent = "X";
      button.addEventListener("click", () => {
        let li = e.target.closest("li");
        let nodes = Array.from(li.closest("ul").children);
        let index = nodes.indexOf(li);

        console.log("Index: " + index);

        let values = formData.getAll("excel-file[]");
        values.splice(index, 1);
        formData.delete("excel-file[]");

        values.forEach((value, _) => {
          formData.append("excel-file[]", value);
        });

        li.remove();
        console.log(formData);
      });

      div.appendChild(button);
      div.appendChild(getFileButton);

      li.appendChild(div);
    });

    console.log(formData);
    updateTotalCount();

    await zipReader.close();
  });

  excelFileInput.addEventListener("change", async (e) => {
    const files = e.target.files;

    for (let i = 0; i < files.length; i++) {
      const file = files[i];
      console.log(`Adding  file to form: ${file.name}`, file);

      formData.append("excel-file[]", file);
      console.log(formData);

      const div = document.createElement("div");

      const li = document.createElement("li");
      const a = document.createElement("a");
      const getFileButton = document.createElement("button");

      getFileButton.textContent = "Get File";
      getFileButton.addEventListener("click", (e) => {
        e.preventDefault();

        const blob = new Blob([e.target.value], { type: e.target.type });
        const url = URL.createObjectURL(blob);
        const a = document.createElement("a");
        a.href = url;
        a.download = file.name;
        a.click();
      });

      li.textContent = file.name;
      excelList.appendChild(li);

      const button = document.createElement("button");
      button.textContent = "X";
      button.addEventListener("click", (e) => {
        let li = e.target.closest("li");
        let nodes = Array.from(li.closest("ul").children);
        let index = nodes.indexOf(li);

        console.log("Index: " + index);

        let values = formData.getAll("excel-file[]");
        values.splice(index, 1);
        formData.delete("excel-file[]");

        values.forEach((value, _) => {
          formData.append("excel-file[]", value);
        });

        // console.log(values);
        console.log(values.map((file) => file.name));

        li.remove();
      });

      div.appendChild(button);
      div.appendChild(getFileButton);

      li.appendChild(div);

      console.log(formData);
    }

      updateTotalCount();
  });


  submitExcelButton.addEventListener("click", async (e) => {
    e.preventDefault();
    loading = true;

    let mark = document.getElementById("mark");
    mark.style.display = "flex";

    // setLoading(true);


    formData.delete("last-mod[]");

        formData.getAll("excel-file[]").map((file) => {
          let lastMod = file.lastModified;
          const date = getDate(lastMod, false);

          return date;
        })

      .forEach((date) => {
        formData.append("last-mod[]", date);
      });

    formData.append("conditions", JSON.stringify(conditionsObject));

    console.log(formData);

    console.log("We're sending a request to the server.");
    const res = await fetch("/api/search", {
      method: "POST",
      body: formData,
    });
    if (!res.ok) {
      mark.style.display = "none";
      const error = await res.json();
      alert(error.error);
      throw error;
    }

    console.log("Received the response, parsing and downloading the output.");

    var date = new Date();
    var time = new Date()
      .toLocaleTimeString("en-US", {
        timeZone: Intl.DateTimeFormat().resolvedOptions().timeZone,
        hour12: false,
        hour: "numeric",
        minute: "numeric",
      })
      .replace(":", "");

    console.log(time);
    const blob = await res.blob();
    const url = window.URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `search${formatDate(
      date,
      "mmddyy"
    )}${time.trim()}.xlsx`;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    window.URL.revokeObjectURL(url);

    console.log("Done!");


    // cleanup
    loading = false;
    mark.style.display = "none";
    excelList.textContent = '';
    formData = new FormData();
    

    // setLoading(loading);
  });

  templateButton.addEventListener("click", async (e) => {
    e.preventDefault();

    let formData = new FormData();
    
    if (conditionsObject.conditions.length == 0) {
      conditionsObject.conditions.push({data: "", title: "", intersections: []});
    }

    formData.append("template", JSON.stringify(conditionsObject));

    const res = await fetch("/api/search/download_template", {
      method: "POST",
      body: formData,
    });
    if (!res.ok) {
      const error = await res.json();
      alert(error.error);
      throw error;
    }

    const blob = await res.blob();
    const url = window.URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = "template.xlsx";
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    window.URL.revokeObjectURL(url);
  });
});
