// Application JavaScript - enough content for gzip to be effective
const App = {
    state: {
        count: 0,
        items: [],
        loading: false,
        error: null,
    },

    init() {
        console.log("App initialized");
        this.bindEvents();
        this.render();
    },

    bindEvents() {
        document.addEventListener("DOMContentLoaded", () => {
            const buttons = document.querySelectorAll("[data-action]");
            buttons.forEach((btn) => {
                btn.addEventListener("click", (e) => {
                    const action = e.target.dataset.action;
                    this.handleAction(action);
                });
            });
        });
    },

    handleAction(action) {
        switch (action) {
            case "increment":
                this.state.count++;
                break;
            case "decrement":
                this.state.count--;
                break;
            case "reset":
                this.state.count = 0;
                break;
            case "addItem":
                this.state.items.push({ id: Date.now(), text: "New Item" });
                break;
            default:
                console.warn("Unknown action:", action);
        }
        this.render();
    },

    render() {
        const counter = document.getElementById("counter");
        if (counter) {
            counter.textContent = this.state.count;
        }

        const itemList = document.getElementById("item-list");
        if (itemList) {
            itemList.innerHTML = this.state.items
                .map(
                    (item) => `
                <li class="item" data-id="${item.id}">
                    ${item.text}
                    <button data-action="removeItem" data-id="${item.id}">Remove</button>
                </li>
            `
                )
                .join("");
        }
    },

    async fetchData(url) {
        this.state.loading = true;
        this.state.error = null;
        this.render();

        try {
            const response = await fetch(url);
            if (!response.ok) {
                throw new Error(`HTTP ${response.status}: ${response.statusText}`);
            }
            const data = await response.json();
            this.state.items = data;
        } catch (err) {
            this.state.error = err.message;
            console.error("Fetch failed:", err);
        } finally {
            this.state.loading = false;
            this.render();
        }
    },
};

App.init();
