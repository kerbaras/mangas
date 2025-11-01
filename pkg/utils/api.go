package utils

import (
	"encoding/json"
	"fmt"
	"net/http"
	"net/url"
)

type API struct {
	client  *http.Client
	baseURL string
}

func NewAPI(baseURL string) *API {
	return &API{client: http.DefaultClient, baseURL: baseURL}
}

func (a *API) Get(path string, params url.Values, v any) error {
	if params != nil {
		path += "?" + params.Encode()
	}
	req, err := http.NewRequest("GET", fmt.Sprintf("%s%s", a.baseURL, path), nil)
	if err != nil {
		return err
	}
	req.Header.Set("Accept", "application/json")
	req.Header.Set("Content-Type", "application/json")
	resp, err := a.client.Do(req)
	if err != nil {
		return err
	}
	defer resp.Body.Close()
	return json.NewDecoder(resp.Body).Decode(v)
}
