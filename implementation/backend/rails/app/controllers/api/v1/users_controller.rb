# frozen_string_literal: true

module Api
  module V1
    class UsersController < ApplicationController
      def index
        users = User.order(:name)
        render json: users
      end

      def show
        user = User.find(params[:id])
        render json: user.as_json(include: :memberships)
      end

      def create
        user = User.new(user_params)
        if user.save
          AuditEvent.create!(action: "create", resource_type: "User", resource_id: user.id)
          render json: user, status: :created
        else
          render json: { errors: user.errors.full_messages }, status: :unprocessable_entity
        end
      end

      def update
        user = User.find(params[:id])
        if user.update(user_params)
          render json: user
        else
          render json: { errors: user.errors.full_messages }, status: :unprocessable_entity
        end
      end

      private

      def user_params
        params.require(:user).permit(:email, :name, :external_id)
      end
    end
  end
end
