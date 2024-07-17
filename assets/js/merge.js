document.addEventListener("DOMContentLoaded", () => {
  /////////// Elements ///////////

  const mainFileList = document.getElementById("main-file-list");
  const mainFileInput = document.getElementById("main-file");
  const zipFileInput = document.getElementById("zip-input");

  const excelList = document.getElementById("excel-list");
  const excelFileInput = document.getElementById("excel-file");
  const submitExcelButton = document.getElementById("submit-excel");
  const cuttingRows = document.getElementById("cutting-rows");

  const sortByDateCheckbox = document.getElementById("sortByDateCheckbox");
  const sortByFileCheckbox = document.getElementById("sortByFileCheckbox");

  let loading = false;

  let mainFileName;

  const formData = new FormData();

  formData.append("last-mod[]", []);

  cuttingRows.value = "";
  /////////// Elements ///////////

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

  sortByDateCheckbox.addEventListener("change", (e) => {
    if (e.target.checked) {
      formData.append("sort_by_date", true);
    } else {
      formData.delete("sort_by_date");
    }
  });

  sortByFileCheckbox.addEventListener("change", (e) => {
    if (e.target.checked) {
      formData.append("sort_by_file", true);
    } else {
      formData.delete("sort_by_file");
    }
  });

  cuttingRows.addEventListener("change", (e) => {
    formData.delete("cuttingRows");
    formData.append("cuttingRows", e.target.value);
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
          let lastMod = file.lastModified;
          const date = getDate(lastMod, false);

          return date;
        })
      )
      .forEach((date) => {
        formData.append("last-mod[]", date);
      });

    formData.append("cuttingRows", cuttingRows.value);

    console.log(formData);

    console.log("We're sending a request to the server.");
    const res = await fetch("/api/merge", {
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
});
