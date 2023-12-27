document.addEventListener("DOMContentLoaded", () => {
  /////////// Elements ///////////
  const mainFileList = document.getElementById("main-file-list");
  const mainFileInput = document.getElementById("main-file");
  const zipFileInput = document.getElementById("zip-input");
  const addMainFileButton = document.getElementById("add-main-file");

  const excelList = document.getElementById("excel-list");
  const excelFileInput = document.getElementById("excel-file");
  const submitExcelButton = document.getElementById("submit-excel");
  const search = document.getElementById("search");
  const searchButton = document.getElementById("add-search");
  const templateButton = document.getElementById("download-template");

  let loading = false;
  let mainFileName;
  const formData = new FormData();

  /////////// ______ ///////////
  let conditionsObject = {
    conditions: [],
  };

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
                <input type="text" placeholder="Title" class="titleInput">
                <input type="text" placeholder="Data" class="dataInput">
                <button class="andButton">And</button>
            </div>
        `;
    document.getElementById("search").appendChild(inputPair);

    // Add a new condition to the formData object
    conditionsObject.conditions.push({
      title: "",
      data: "",
      intersections: [],
    });
  });

  document.addEventListener("click", function (e) {
    if (e.target && e.target.classList.contains("andButton")) {
      e.preventDefault();

      let inputPair = document.createElement("div");
      inputPair.className = "pairContainer";
      inputPair.style.display = "flex";
      inputPair.innerHTML = `
                <input type="text" placeholder="Title" class="intersectionTitleInput">
                <input type="text" placeholder="Data" class="intersectionDataInput">
                <button class="andButton">And</button>
            `;
      e.target.parentNode.after(inputPair);
      e.target.remove();

      // Add a new intersection to the last condition in the formData object
      conditionsObject.conditions[
        conditionsObject.conditions.length - 1
      ].intersections.push({
        title: "",
        data: "",
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
  });

  mainFileInput.addEventListener("change", async (e) => {
    const files = mainFileInput.files;
    for (let i = 0; i < files.length; i++) {
      const file = files[i];

      formData.append("main-file", file);

      console.log(formData);

      const li = document.createElement("li");

      mainFileName = file.name;
      li.textContent = file.name;
      mainFileList.appendChild(li);

      const button = document.createElement("button");
      button.textContent = "X";
      button.addEventListener("click", () => {
        li.remove();
        formData.delete("main-file");
        console.log(formData);
      });

      li.appendChild(button);
    }
  });

  submitExcelButton.addEventListener("click", async (e) => {
    e.preventDefault();
    loading = true;
    // setLoading(true);

    let mainFile = formData.get("main-file");
    let mainFileDate = [];

    if (mainFile) {
      mainFileDate.push(getDate(mainFile.lastModified, false));
    }

    console.log("mainFileDate: " + mainFileDate);

    formData.delete("last-mod[]");

    mainFileDate
      .concat(
        formData.getAll("excel-file[]").map((file) => {
          console.log("nigga we here");

          let lastMod = file.lastModified;
          const date = getDate(lastMod, false);

          return date;
        })
      )
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
    a.download = `${mainFileName.replace(".xlsx", "")}-merge${formatDate(
      date,
      "mmddyy"
    )}${time.trim()}.xlsx`;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    window.URL.revokeObjectURL(url);

    console.log("Done!");

    loading = false;
    // setLoading(loading);
  });

  templateButton.addEventListener("click", async (e) => {
    e.preventDefault();

    let formData = new FormData();
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
